use anyhow::Context;
use clap::Parser;
use reverse_depends_ng_poc::{Args, detect_devel_release, fetch_binaries, fetch_sources};

const USER_AGENT: &str = concat!("reverse-depends/", env!("CARGO_PKG_VERSION"));

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
    // TODO debug
    dbg!(&args);

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

    // TODO debug
    dbg!(&source_packages);

    // If searching for binary packages isn't necessary, then no
    // searches will be made within fetch_binaries().
    let binary_packages = fetch_binaries(&client, release, &args)
        .await
        .with_context(|| "Failed to fetch binaries")?;

    // TODO debug
    dbg!(&binary_packages);

    todo!()
}
