//! This module is responsible for all functionality related to the
//! caching of archive data.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Increment this whenever the cache file layout changes so stale
/// entries are automatically invalidated.
const CACHE_VERSION: u32 = 1;

/// A single entry for the cache, containing its layout version and HTTP
/// `ETag`.
#[derive(Debug, Clone, Deserialize)]
struct CacheEntry<T> {
    version: u32,
    etag: Option<String>,
    data: T,
}

/// Separate serialization-side struct so `data` can be passed by
/// reference without requiring a clone.
#[derive(Debug, Clone, Serialize)]
struct CacheEntryRef<'a, T> {
    version: u32,
    etag: Option<&'a str>,
    data: &'a T,
}

/// Load a cached entry for `archive_url`. Returns `None` if there is no
/// cache file, if the stored version doesn't match [`CACHE_VERSION`], or
/// if deserialization fails.
///
/// All I/O errors are silently ignored.
pub(crate) fn load_cache<T: for<'de> Deserialize<'de>>(
    archive_url: &str,
) -> Option<(Option<String>, T)> {
    let path = cache_path(archive_url)?;
    let bytes = std::fs::read(path).ok()?;
    let entry: CacheEntry<T> = postcard::from_bytes(&bytes).ok()?;
    if entry.version != CACHE_VERSION {
        return None;
    }
    Some((entry.etag, entry.data))
}

/// Persist `data` for `archive_url` to the cache, tagged with `etag`.
///
/// All I/O errors are silently ignored -- a broken cache should never
/// crash the tool.
pub(crate) fn save_cache<T: Serialize>(archive_url: &str, etag: Option<&str>, data: &T) {
    let Some(path) = cache_path(archive_url) else {
        eprintln!("Warning: failed to determine cache path; is $XDG_CACHE_HOME or $HOME set?");
        return;
    };

    if let Some(parent) = path.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        eprintln!(
            "Warning: failed to create cache parent dir {}",
            parent.display()
        );
        return;
    }

    let entry = CacheEntryRef {
        version: CACHE_VERSION,
        etag,
        data,
    };
    let Ok(bytes) = postcard::to_allocvec(&entry) else {
        return;
    };
    if let Err(e) = std::fs::write(path, bytes) {
        eprintln!("{e}");
    }
}

/// The path of the cache directory. Compliant with the
/// [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
/// by prioritizing `$XDG_CACHE_HOME` as the base and using `~/.cache`
/// as a fallback.
fn cache_dir() -> Option<PathBuf> {
    let base = if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(xdg)
    } else {
        PathBuf::from(std::env::var_os("HOME")?).join(".cache")
    };
    Some(base.join(env!("CARGO_PKG_NAME")))
}

/// Generates the path to this particular cache file by turning the
/// archive URL into a filesystem-friendly name.
fn cache_path(archive_url: &str) -> Option<PathBuf> {
    let sanitized = archive_url
        .strip_prefix("https://")
        .or_else(|| archive_url.strip_prefix("http://"))
        .unwrap_or(archive_url)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    Some(cache_dir()?.join(sanitized))
}
