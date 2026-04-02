use std::collections::{HashMap, HashSet};

use anyhow::{Context, bail};
use clap::Parser;
use reverse_depends_ng_poc::{
    Args, binaries_provides, detect_devel_release, fetch_binaries, fetch_sources, find_rev_deps,
    find_rev_deps_recursive, list_output, list_output_recursive, source_binaries, verbose_output,
    verbose_output_recursive,
};

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let args = Args::parse();
    if let Err(e) = run(args).await {
        // anyhow's `{e:#}` format prints the full error chain.
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

async fn run(args: Args) -> anyhow::Result<()> {
    // If the user didn't specify a release, try to determine the
    // current devel release using `distro-info`.
    let release = match &args.release {
        Some(r) => r,
        None => &detect_devel_release()?,
    };

    let client = reqwest::Client::builder()
        .no_gzip()
        .user_agent(USER_AGENT)
        .build()?;

    let source_packages = if args.need_source_packages() {
        fetch_sources(&client, release, &args)
            .await
            .with_context(|| "Failed to fetch sources")?
    } else {
        Vec::new()
    };

    // If searching for binary packages isn't necessary, then no
    // searches will be made within fetch_binaries().
    let binary_packages = fetch_binaries(&client, release, &args)
        .await
        .with_context(|| "Failed to fetch binaries")?;

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

    if args.recursive {
        let mut all_results = find_rev_deps_recursive(
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

        if all_results.get(raw_name).is_none_or(HashMap::is_empty) {
            print_no_rev_deps(&args.package);
            return Ok(());
        }

        if args.list {
            println!("{}", list_output_recursive(&all_results));
        } else {
            println!("{}", verbose_output_recursive(raw_name, &all_results));
        }
    } else {
        let mut rev_deps = find_rev_deps(&binary_packages, &source_packages, &target_refs, &args);

        if !args.components.is_empty() {
            let allowed: HashSet<_> = args.components.iter().cloned().collect();
            for entries in rev_deps.values_mut() {
                entries.retain(|e| allowed.contains(e.component));
            }
            rev_deps.retain(|_, v| !v.is_empty());
        }

        if rev_deps.is_empty() {
            print_no_rev_deps(&args.package);
            return Ok(());
        }

        if args.list {
            println!("{}", list_output(&rev_deps));
        } else {
            println!("{}", verbose_output(raw_name, &rev_deps));
        }
    }

    Ok(())
}

// Helper function to print msg when no reverse deps are found.
fn print_no_rev_deps(package: &str) {
    eprintln!("No reverse dependencies found for '{}'.", package);
}
