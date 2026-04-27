//! This module is used to get info about the system.

use anyhow::{Context, anyhow};

use crate::Result;

/// Detect the current development release by using
/// [`distro-info(1)`](https://manpages.debian.org/unstable/distro-info/distro-info.1.en.html).
///
/// # Errors
///
/// This function returns a [`crate::Error`] if the underlying
/// `distro-info` command fails.
pub fn detect_devel_release() -> Result<String> {
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
