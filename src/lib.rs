//! # reverse-depends-ng-poc
//!
//! Proof of concept for a modernized reverse-depends.
#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    clippy::all,
    clippy::pedantic,
    clippy::todo
)]

use anyhow::{Context, anyhow};

mod archive;
mod args;
mod cache;
mod output;
mod parsing;
mod resolver;
mod vendor;

pub use archive::*;
pub use args::*;
pub(crate) use cache::{load_cache, save_cache};
pub use output::*;
pub use parsing::*;
pub use resolver::*;
pub use vendor::*;

/// Detect the current development release by using
/// [`distro-info(1)`](https://manpages.debian.org/unstable/distro-info/distro-info.1.en.html).
///
/// # Errors
///
/// Return an [`anyhow::Error`] if the underlying `distro-info` command
/// fails, returns a non-zero exit code, or produces invalid UTF-8
/// output.
pub fn detect_devel_release() -> anyhow::Result<String> {
    let output = std::process::Command::new("distro-info")
        .arg("--devel")
        .output()
        .context("Failed to execute `distro-info`. Is the `distro-info` package installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "`distro-info --devel` failed with status {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .context("Failed to parse `distro-info` output as valid UTF-8")?;

    Ok(stdout.trim().to_string())
}
