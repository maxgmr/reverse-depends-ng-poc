//! Module for reverse-dependency resolution. Works on lists of parsed
//! archive data, i.e., slices of [`BinaryPackage`]s and
//! [`SourcePackage`]s.

use std::collections::HashSet;

use crate::{BinaryPackage, SourcePackage};

/// Return all binary package names produced by the source package with
/// the given name.
#[must_use]
pub fn source_binaries(sources: &[SourcePackage], source_name: &str) -> HashSet<String> {
    sources
        .iter()
        .filter(|s| s.name == source_name)
        .flat_map(|s| s.binaries.iter().cloned())
        .collect()
}
