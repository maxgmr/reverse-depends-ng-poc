use clap::Parser;
use reverse_depends_ng_poc::{Args, detect_devel_release};

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
    dbg!(&args);
    todo!()
}
