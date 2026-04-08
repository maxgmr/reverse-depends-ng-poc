//! All functionality related to querying the archive.

use std::{io::Read, sync::Arc};

use crate::{
    Args, BinaryPackage, SourcePackage, load_cache, parse_binary_packages, parse_source_packages,
    save_cache,
};

use anyhow::Context;
use flate2::read::GzDecoder;
use futures::future::join_all;
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

/// Max number of requests at one time.
const MAX_CONCURRENT: usize = 16;

/// The different types of responses from an archive query.
enum FetchResult {
    /// HTTP 200: decompressed body and optional ETag from response
    Fresh { text: String, etag: Option<String> },
    /// HTTP 304: the server confirmed the cached copy is still current
    NotModified,
    /// HTTP 404: resource does not exist for this release/component
    NotFound,
}

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
    client: &ClientWithMiddleware,
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
            let cache = args.cache;

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");

                fetch_parsed_cached(&client, &url, cache, |text| {
                    parse_source_packages(text, component, pocket).with_context(|| {
                        format!("Failed to parse Sources.gz for {component} in {pocket}")
                    })
                })
                .await
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
    client: &ClientWithMiddleware,
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
                let cache = args.cache;

                handles.push(tokio::spawn(async move {
                    let _permit = sem.acquire().await.expect("semaphore closed");
                    fetch_parsed_cached(&client, &url, cache, |text| {
                        parse_binary_packages(text, arch, component, pocket).with_context(|| {
                            format!("Failed to parse Packages.gz for {component} in {pocket} for arch {arch}")
                        })
                    })
                    .await
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

/// Fetch a `.gz` archive URL, parse it with `parse`, and cache the
/// result. On subsequent calls the cached copy is returned immediately
/// if the server response with 304 Not Modified.
///
/// When `cache` is `false`, the on-disk cache is not read but a fresh
/// result is still written so the cache stays warm for future runs.
async fn fetch_parsed_cached<T, F>(
    client: &ClientWithMiddleware,
    url: &str,
    cache: bool,
    parse: F,
) -> anyhow::Result<Vec<T>>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: FnOnce(&str) -> anyhow::Result<Vec<T>>,
{
    let cached = if cache {
        load_cache::<Vec<T>>(url)
    } else {
        None
    };
    let cached_etag = cached
        .as_ref()
        .and_then(|(etag, _): &(Option<String>, Vec<T>)| etag.as_deref());

    match fetch_gz_conditional(client, url, cached_etag)
        .await
        .with_context(|| format!("Failed to fetch {url}"))?
    {
        FetchResult::NotFound => Ok(Vec::new()),
        FetchResult::NotModified => Ok(cached.unwrap().1),
        FetchResult::Fresh { text, etag } => {
            let data = parse(&text)?;
            save_cache(url, etag.as_deref(), &data);
            Ok(data)
        }
    }
}

/// Perform a GET for a `.gz` URL, optionally sending `If-None-Match`
/// when `etag` is [`Some`].
async fn fetch_gz_conditional(
    client: &ClientWithMiddleware,
    url: &str,
    etag: Option<&str>,
) -> anyhow::Result<FetchResult> {
    let mut req = client.get(url);
    if let Some(etag) = etag {
        req = req.header(reqwest::header::IF_NONE_MATCH, etag);
    }
    let response = req.send().await.with_context(|| format!("GET {url}"))?;

    match (response.status().is_success(), response.status().as_u16()) {
        (_, 304) => return Ok(FetchResult::NotModified),
        (_, 404) => return Ok(FetchResult::NotFound),
        (false, code) => anyhow::bail!("HTTP {code} for {url}"),
        _ => {}
    }

    // Fresh result, get ETag and decode the GZ data

    let etag = response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    if etag.is_none() {
        eprintln!("Warning: {url} did not return an ETag header");
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

    Ok(FetchResult::Fresh { text, etag })
}
