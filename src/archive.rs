//! All functionality related to querying the archive.

use std::sync::Arc;

use crate::{Args, SourcePackage};

use reqwest::Client;
use tokio::sync::Semaphore;

/// Max number of requests at one time.
const MAX_CONCURRENT: usize = 16;

pub async fn fetch_sources(
    client: &Client,
    release: &str,
    args: &Args,
) -> anyhow::Result<Vec<SourcePackage>> {
    let sem = Arc::new(Semaphore::new(MAX_CONCURRENT));
    todo!();
}
