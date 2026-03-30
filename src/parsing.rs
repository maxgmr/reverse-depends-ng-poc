//! This module contains all functionality related to parsing package
//! info from archive data.

use anyhow::Context;
use deb822_fast::borrowed::{BorrowedParagraph, BorrowedParser};

/// A source package from the archive along with all its build dependencies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourcePackage {
    /// The source package's name.
    pub name: String,
    /// The archive component.
    pub component: &'static str,
    /// The pocket.
    pub pocket: &'static str,
    /// The raw `Binaries` field.
    pub binaries: String,
    /// The raw `Build-Depends` field.
    pub build_depends: String,
    /// The raw `Build-Depends-Indep` field.
    pub build_depends_indep: String,
    /// The raw `Build-Depends-Arch` field.
    pub build_depends_arch: String,
}

/// A binary package from the archive along with all its package
/// relationships.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryPackage {
    /// The binary package's name.
    pub name: String,
    /// The architecture of the `Packages.gz` file from which this
    /// binary package came.
    pub arch: &'static str,
    /// The archive component.
    pub component: &'static str,
    /// The pocket.
    pub pocket: &'static str,
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
/// - The final list of source packages is empty, meaning there was a
///   problem with parsing the text.
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
    // TODO potential optimization: zero-copy?
    let source_packages: Vec<SourcePackage> = paragraphs
        .into_iter()
        .filter_map(|paragraph| {
            Some(SourcePackage {
                name: paragraph.get_single("package")?.to_string(),
                component,
                pocket,
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
            })
        })
        .collect();

    if source_packages.is_empty() {
        anyhow::bail!(
            "List for component {component} and pocket {pocket} is empty; there was a problem parsing the text"
        );
    }

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
/// - The final list of binary packages is empty, meaning there was a
///   problem with parsing the text.
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
    // TODO potential optimization: zero-copy?
    let binary_packages: Vec<BinaryPackage> = paragraphs
        .into_iter()
        .filter_map(|paragraph| {
            Some(BinaryPackage {
                name: paragraph.get_single("package")?.to_string(),
                arch,
                component,
                pocket,
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

    if binary_packages.is_empty() {
        anyhow::bail!(
            "List for component {component} and pocket {pocket} for arch {arch} is empty; there was a problem parsing the text"
        );
    }

    Ok(binary_packages)
}

/// Helper function which gets a field from the given paragraph,
/// splitting the field entries by commas or returning [`None`] if the
/// given field does not exist.
fn field_to_vec(paragraph: &BorrowedParagraph<'_>, field: &str) -> Option<Vec<String>> {
    Some(
        paragraph
            .get_single(field)?
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}
