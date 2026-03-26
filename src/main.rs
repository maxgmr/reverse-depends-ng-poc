use clap::Parser;
use reverse_depends_ng_poc::Args;

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
    todo!()
}
