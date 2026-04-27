use std::time::Duration;

use ahash::AHashSet as HashSet;
use anyhow::{Context, bail};
use clap::Parser;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};

use reverse_depends_ng_poc::{
    Args, Result, ReverseIndex, binaries_provides, detect_devel_release, fetch_binaries,
    fetch_sources, find_rev_deps, find_rev_deps_recursive, list_output, list_output_recursive,
    source_binaries, verbose_output, verbose_output_recursive,
};

#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
const DEFAULT_MAX_RETRIES: u32 = 5;
const DEFAULT_MIN_DELAY: Duration = Duration::from_millis(500);
const DEFAULT_MAX_DELAY: Duration = Duration::from_secs(5);

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let args = Args::parse();
    match run(args).await {
        Ok(true) => {}                      // exit 0 - results found
        Ok(false) => std::process::exit(1), // exit 1 - no results
        Err(e) => {
            // anyhow's `{e:#}` format prints the full error chain.
            eprintln!("error: {e:#}");
            std::process::exit(2); // exit 2 - fatal error
        }
    }
}

/// This function returns `Ok(true)` if results were found and
/// `Ok(false)` if no results were found.
async fn run(args: Args) -> Result<bool> {
    // If the user didn't specify a release, try to determine the
    // current devel release using `distro-info`.
    let release: &str = match &args.release {
        Some(r) => r,
        None => &detect_devel_release()?,
    };

    let retry_policy = ExponentialBackoff::builder()
        .retry_bounds(DEFAULT_MIN_DELAY, DEFAULT_MAX_DELAY)
        .build_with_max_retries(DEFAULT_MAX_RETRIES);
    let client = ClientBuilder::new(
        reqwest::Client::builder()
            .no_gzip()
            .user_agent(USER_AGENT)
            .build()?,
    )
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .build();

    let (source_packages, binary_packages) = tokio::try_join!(
        async {
            if args.need_source_packages() {
                fetch_sources(&client, release, &args)
                    .await
                    .with_context(|| "Failed to fetch sources")
            } else {
                Ok(Vec::new())
            }
        },
        async {
            // If searching for binary packages isn't necessary, then no
            // searches will be made within fetch_binaries().
            fetch_binaries(&client, release, &args)
                .await
                .with_context(|| "Failed to fetch binaries")
        },
    )?;

    // Expand the name in two possible ways:
    //  1. If 'src:' prefix, then replace with all binary names for
    //     that source package.
    //  2. If checking for Provides relationships, then add all virtual
    //     names the target provides.
    let (is_src, raw_name) = match args.package.strip_prefix("src:") {
        Some(name) => (true, name),
        None => (false, args.package.as_str()),
    };
    let mut target_names: HashSet<String> = if is_src {
        let bins = source_binaries(&source_packages, raw_name);
        if bins.is_empty() {
            bail!("source package '{raw_name}' not found in release '{release}'");
        }
        bins
    } else {
        let mut s = HashSet::new();
        s.insert(raw_name.to_string());
        s
    };
    if args.provides {
        let provided = binaries_provides(&binary_packages, &target_names);
        target_names.extend(provided);
    }

    let target_refs: HashSet<&str> = target_names.iter().map(String::as_str).collect();

    // Pre-compute all reverse dependencies for more efficient recursion
    let index = ReverseIndex::build(&binary_packages, &source_packages);

    if args.recursive {
        let mut all_results = find_rev_deps_recursive(
            &index,
            &binary_packages,
            &source_packages,
            raw_name,
            &target_refs,
            &args,
        );

        if !args.components.is_empty() {
            let allowed: HashSet<_> = args.components.iter().cloned().collect();
            for inner in all_results.values_mut() {
                for entries in inner.values_mut() {
                    entries.retain(|e| allowed.contains(e.component));
                }
                inner.retain(|_, entries| !entries.is_empty());
            }
            all_results.retain(|_, inner| !inner.is_empty());
        }

        if all_results.get(raw_name).is_none_or(|m| m.is_empty()) {
            print_no_rev_deps(&args.package);
            return Ok(false);
        }

        if args.list {
            println!("{}", list_output_recursive(&all_results));
        } else {
            println!("{}", verbose_output_recursive(raw_name, &all_results));
        }
    } else {
        let mut rev_deps = find_rev_deps(&index, &target_refs, &args);

        if !args.components.is_empty() {
            let allowed: HashSet<_> = args.components.iter().cloned().collect();
            for entries in rev_deps.values_mut() {
                entries.retain(|e| allowed.contains(e.component));
            }
            rev_deps.retain(|_, v| !v.is_empty());
        }

        if rev_deps.is_empty() {
            print_no_rev_deps(&args.package);
            return Ok(false);
        }

        if args.list {
            println!("{}", list_output(&rev_deps));
        } else {
            println!("{}", verbose_output(raw_name, &rev_deps));
        }
    }

    Ok(true)
}

// Helper function to print msg when no reverse deps are found.
fn print_no_rev_deps(package: &str) {
    eprintln!("No reverse dependencies found for '{}'.", package);
}
