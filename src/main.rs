use clap::Parser;
use reverse_depends_ng_poc::{Cli, detect_devel_release};

const DEVEL_FALLBACK: &str = "unstable";

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli_args = Cli::parse();

    let release = cli_args.release.unwrap_or_else(|| {
        detect_devel_release().unwrap_or_else(|| {
            eprintln!("Warning: could not detect devel release; defaulting to '{DEVEL_FALLBACK}'");
            DEVEL_FALLBACK.to_string()
        })
    });

    println!("{release}");
}
