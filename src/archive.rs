//! All functionality related to querying the archive.

use std::{io::Read, sync::Arc};

use crate::{Args, BinaryPackage, SourcePackage, parse_binary_packages, parse_source_packages};

use anyhow::Context;
use flate2::read::GzDecoder;
use futures::future::join_all;
use reqwest::Client;
use tokio::sync::Semaphore;

/// Max number of requests at one time.
const MAX_CONCURRENT: usize = 16;

/// Fetch the source packages of the given component and pocket combos
/// for the given distro release. If successful, this function will
/// return a list of [`SourcePackage`]s, which denote all the source
/// packages and their build dependencies.
///
/// # Errors
///
/// This function returns an [`anyhow::Error`] in the following
/// situations:
///
/// - No valid components or pockets for the given distro were given.
/// - There was a failure fetching Sources.gz from the distro archive.
/// - The downloaded Sources.gz was an invalid format; i.e., not
///   [DEB822](https://repolib.readthedocs.io/en/latest/deb822-format.html).
/// - A Tokio task panicked or got cancelled.
#[allow(clippy::missing_panics_doc)]
pub async fn fetch_sources(
    client: &Client,
    release: &str,
    args: &Args,
) -> anyhow::Result<Vec<SourcePackage>> {
    let sem = Arc::new(Semaphore::new(MAX_CONCURRENT));
    let mut handles = Vec::new();

    let archive_base = args.vendor.archive();

    for pocket in args.selected_pockets()? {
        for component in args.selected_components()? {
            let url =
                format!("{archive_base}/dists/{release}{pocket}/{component}/source/Sources.gz");
            let client = client.clone();
            let sem = Arc::clone(&sem);

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");

                let text = fetch_gz(&client, &url)
                    .await
                    .with_context(|| format!("Failed to fetch Sources.gz from {url}"))?;
                // If text is empty, then that component just doesn't
                // exist for that release.
                if text.is_empty() {
                    return Ok(Vec::new());
                }

                let packages =
                    parse_source_packages(&text, component, pocket).with_context(|| {
                        format!("Failed to parse downloaded Sources.gz for {component} in {pocket}")
                    })?;

                Ok::<_, anyhow::Error>(packages)
            }));
        }
    }

    let mut all_packages = Vec::new();

    for handle_result in join_all(handles).await {
        let inner_result = handle_result.context("Tokio task panicked or was cancelled")?;
        let packages = inner_result?;
        all_packages.extend(packages);
    }

    Ok(all_packages)
}

/// Fetch the binary packages of the given component, pocket, and arch
/// combos for the given distro release. If successful, this function
/// will return a list of [`BinaryPackage`]s, which denote all the
/// binary packages and their dependencies.
///
/// # Errors
///
/// This function returns an [`anyhow::Error`] in the following
/// situations:
///
/// - No valid components or pockets for the given distro were given.
/// - There was a failure fetching Packages.gz from the distro archive.
/// - The downloaded Packages.gz was an invalid format; i.e., not
///   [DEB822](https://repolib.readthedocs.io/en/latest/deb822-format.html).
/// - A Tokio task panicked or got cancelled.
#[allow(clippy::missing_panics_doc)]
pub async fn fetch_binaries(
    client: &Client,
    release: &str,
    args: &Args,
) -> anyhow::Result<Vec<BinaryPackage>> {
    let search_combos = args.needed_arch_searches(release);
    if search_combos.is_empty() {
        return Ok(Vec::new());
    }

    let sem = Arc::new(Semaphore::new(MAX_CONCURRENT));
    let mut handles = Vec::new();

    for pocket in args.selected_pockets()? {
        for component in args.selected_components()? {
            for search_combo in &search_combos {
                let archive_base = search_combo.base_url;
                let arch = search_combo.arch;
                let url = format!(
                    "{archive_base}/dists/{release}{pocket}/{component}/binary-{arch}/Packages.gz"
                );
                let client = client.clone();
                let sem = Arc::clone(&sem);

                handles.push(tokio::spawn(async move {
                    let _permit = sem.acquire().await.expect("semaphore closed");

                    let text = fetch_gz(&client, &url)
                        .await
                        .with_context(|| format!("Failed to fetch Packages.gz from {url}"))?;
                    // If text is empty, then that component just
                    // doesn't exist for that release.
                    if text.is_empty() {
                        return Ok(Vec::new());
                    }

                    let packages = parse_binary_packages(&text, arch, component, pocket).with_context(|| { format!("Failed to parse downloaded Packages.gz for {component} in {pocket} for arch {arch}") })?;

                    Ok::<_, anyhow::Error>(packages)
                }));
            }
        }
    }

    let mut all_packages = Vec::new();

    for handle_result in join_all(handles).await {
        let inner_result = handle_result.context("Tokio task panicked or was cancelled")?;
        let packages = inner_result?;
        all_packages.extend(packages);
    }

    Ok(all_packages)
}

/// Download a `.gz` URL and return its decompressed content as a
/// [`String`], returning an empty [`String`] if HTTP 404 is returned.
async fn fetch_gz(client: &Client, url: &str) -> anyhow::Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?;

    match (response.status().is_success(), response.status().as_u16()) {
        // Return an empty string if 404
        (false, 404) => return Ok(String::new()),

        // Return an error if any other error
        (false, _) => anyhow::bail!("HTTP {} for {url}", response.status()),
        // Continue if success
        (true, _) => (),
    }

    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("Failed to read body of {url}"))?;

    let mut decoder = GzDecoder::new(bytes.as_ref());
    let mut text = String::new();
    decoder
        .read_to_string(&mut text)
        .with_context(|| format!("Failed to decompress {url}"))?;

    Ok(text)
}
