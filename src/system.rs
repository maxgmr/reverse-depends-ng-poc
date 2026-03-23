//! Miscallaneous helpers which get system info.

/// Detect the current development release by using
/// [`distro-info(1)`](https://manpages.debian.org/unstable/distro-info/distro-info.1.en.html).
///
/// Returns [`None`] if the underlying `distro-info` command fails.
#[must_use]
pub fn detect_devel_release() -> Option<String> {
    std::process::Command::new("distro-info")
        .arg("--devel")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}
