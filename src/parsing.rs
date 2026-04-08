//! This module contains all functionality related to parsing package
//! info from archive data.

use anyhow::Context;
use deb822_fast::borrowed::BorrowedParser;
use serde::{Deserialize, Serialize};

/// A source package from the archive along with all its build dependencies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePackage {
    /// The source package's name.
    pub name: String,
    /// The archive component.
    pub component: String,
    /// The pocket.
    pub pocket: String,
    /// The raw `Binaries` field.
    pub binaries: String,
    /// The raw `Build-Depends` field.
    pub build_depends: String,
    /// The raw `Build-Depends-Indep` field.
    pub build_depends_indep: String,
    /// The raw `Build-Depends-Arch` field.
    pub build_depends_arch: String,
    /// The raw `Testsuite-Triggers` field.
    pub testsuite_triggers: String,
}

/// A binary package from the archive along with all its package
/// relationships.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryPackage {
    /// The binary package's name.
    pub name: String,
    /// The architecture of the `Packages.gz` file from which this
    /// binary package came.
    pub arch: String,
    /// The archive component.
    pub component: String,
    /// The pocket.
    pub pocket: String,
    /// The raw `Depends` field.
    pub depends: String,
    /// The raw `Pre-Depends` field.
    pub pre_depends: String,
    /// The raw `Recommends` field.
    pub recommends: String,
    /// The raw `Suggests` field.
    pub suggests: String,
    /// The raw `Provides` field.
    pub provides: String,
}

/// Parse the raw text content as a
/// [DEB822 format](https://repolib.readthedocs.io/en/latest/deb822-format.html)
/// list of source packages, returning a list of [`SourcePackage`]s if
/// successful.
///
/// This function is lossy, meaning it ignores any invalid DEB822 lines
/// or package paragraphs with no name or binary packages.
///
/// # Errors
///
/// This function returns an [`anyhow::Error`] in the following
/// situations:
///
/// - The text is unparseable in the DEB822 format.
pub fn parse_source_packages(
    content: &str,
    component: &'static str,
    pocket: &'static str,
) -> anyhow::Result<Vec<SourcePackage>> {
    let paragraphs = BorrowedParser::new(content)
        .parse_all()
        .with_context(|| "Failed to parse deb822 format")?;
    let source_packages: Vec<SourcePackage> = paragraphs
        .into_iter()
        .filter_map(|paragraph| {
            Some(SourcePackage {
                name: paragraph.get_single("package")?.to_string(),
                component: component.to_string(),
                pocket: pocket.to_string(),
                binaries: paragraph.get_single("binary")?.to_string(),
                build_depends: paragraph
                    .get_single("build-depends")
                    .unwrap_or_default()
                    .to_string(),
                build_depends_indep: paragraph
                    .get_single("build-depends-indep")
                    .unwrap_or_default()
                    .to_string(),
                build_depends_arch: paragraph
                    .get_single("build-depends-arch")
                    .unwrap_or_default()
                    .to_string(),
                testsuite_triggers: paragraph
                    .get_single("testsuite-triggers")
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect();

    Ok(source_packages)
}

/// Parse the raw text content as a
/// [DEB822 format](https://repolib.readthedocs.io/en/latest/deb822-format.html)
/// list binary packages, returning a list of [`BinaryPackage`]s if
/// successful.
///
/// This function is lossy, meaning it ignores any invalid DEB822 lines
/// or package paragraphs with no name.
///
/// # Errors
///
/// This function returns an [`anyhow::Error`] in the following
/// situations:
///
/// - The text is unparseable in the DEB822 format.
pub fn parse_binary_packages(
    content: &str,
    arch: &'static str,
    component: &'static str,
    pocket: &'static str,
) -> anyhow::Result<Vec<BinaryPackage>> {
    let paragraphs = BorrowedParser::new(content)
        .parse_all()
        .with_context(|| "Failed to parse deb822 format")?;
    let binary_packages: Vec<BinaryPackage> = paragraphs
        .into_iter()
        .filter_map(|paragraph| {
            Some(BinaryPackage {
                name: paragraph.get_single("package")?.to_string(),
                arch: arch.to_string(),
                component: component.to_string(),
                pocket: pocket.to_string(),
                depends: paragraph
                    .get_single("depends")
                    .unwrap_or_default()
                    .to_string(),
                pre_depends: paragraph
                    .get_single("pre-depends")
                    .unwrap_or_default()
                    .to_string(),
                recommends: paragraph
                    .get_single("recommends")
                    .unwrap_or_default()
                    .to_string(),
                suggests: paragraph
                    .get_single("suggests")
                    .unwrap_or_default()
                    .to_string(),
                provides: paragraph
                    .get_single("provides")
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect();

    Ok(binary_packages)
}

/// Parse a raw dependency string into a list of OR-groups, stripping
/// out the version constraints and architecture restrictions, as only
/// the package names themselves are needed for reverse-dep lookup.
#[must_use]
pub fn parse_dep_names(raw_field: &str) -> Vec<Vec<&str>> {
    raw_field
        .split(',')
        .map(|and_group| {
            and_group
                .split('|')
                .filter_map(extract_name)
                .collect::<Vec<_>>()
        })
        .filter(|group| !group.is_empty())
        .collect()
}

/// Parse a raw dependency string into OR-groups, returning each group
/// as a `(raw_group, extracted_names)` pair. The raw-group string is
/// borrowed directly from the input, retaining version constraints and
/// architecture restrictions; only the name list is stripped.
///
/// [`parse_dep_names`] is used when only the name lists are needed.
#[must_use]
pub fn parse_dep_groups(raw_field: &str) -> Vec<(&str, Vec<&str>)> {
    raw_field
        .split(',')
        .filter_map(|and_group| {
            let names: Vec<&str> = and_group.split('|').filter_map(extract_name).collect();
            if names.is_empty() {
                None
            } else {
                Some((and_group.trim(), names))
            }
        })
        .collect()
}

/// Parse a raw `Provides` field into a list of packages, stripping out
/// the version constraints and architecture restrictions, as only the
/// package names themselves are needed for reverse-dep lookup.
pub fn parse_provides(raw: &str) -> Vec<&str> {
    raw.split(',').filter_map(extract_name).collect()
}

/// Extract the package name from a raw dependency token; i.e., remove
/// version constraints and architecture restrictions.
///
/// Returns [`Option::None`] if the token is empty.
#[must_use]
pub fn extract_name(raw: &str) -> Option<&str> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    let end = s
        .find(|c: char| c.is_whitespace() || c == '(' || c == '[' || c == ':')
        .unwrap_or(s.len());
    let name = &s[..end];
    if name.is_empty() { None } else { Some(name) }
}

// AI-generated unit tests
#[cfg(test)]
#[path = "unit_tests/parsing_tests.rs"]
mod tests;
