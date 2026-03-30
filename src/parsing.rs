//! This module contains all functionality related to parsing package
//! info from archive data.

use anyhow::Context;
use deb822_fast::borrowed::{BorrowedParagraph, BorrowedParser};

/// A source package from the archive along with all its build dependencies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourcePackage {
    /// The name of this source package.
    pub name: String,
    /// The archive component of this source package.
    pub component: &'static str,
    /// The pocket of this source package.
    pub pocket: &'static str,
    /// The binary packages built by this source package.
    pub binaries: Vec<String>,
    /// The binary packages required to build any part of this source
    /// package.
    pub build_depends: Vec<String>,
    /// The binary packages required to build arch-independent binary
    /// packages provided by this source package.
    pub build_depends_indep: Vec<String>,
    /// The binary packages required to build arch-dependent binary
    /// packages provided by this source package.
    pub build_depends_arch: Vec<String>,
}

/// A binary package from the archive along with all its package
/// relationships.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryPackage {
    /// The name of this binary package.
    pub name: String,
    /// The architecture of the `Packages.gz` file from which this
    /// binary package came.
    pub arch: &'static str,
    /// The archive component of this source package.
    pub component: &'static str,
    /// The pocket of this source package.
    pub pocket: &'static str,
    /// The packages upon which this binary package depends.
    pub depends: Vec<String>,
    /// The packages which must be fully installed before this
    /// package's installation can begin.
    pub pre_depends: Vec<String>,
    /// The packages upon which this package has a strong, but not
    /// absolute, dependency.
    pub recommends: Vec<String>,
    /// The packages which enhance this package's functionality.
    pub suggests: Vec<String>,
    /// The virtual package names this package satisfies.
    pub provides: Vec<String>,
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
    // TODO optimization: don't waste time formatting the fields of
    // packages you won't even be looking at. Leave the SourcePackage
    // fields as their raw versions and split them later.
    let source_packages: Vec<SourcePackage> = paragraphs
        .into_iter()
        .filter_map(|paragraph| {
            Some(SourcePackage {
                name: paragraph.get_single("package")?.to_string(),
                component,
                pocket,
                binaries: field_to_vec(&paragraph, "binary")?,
                build_depends: field_to_vec(&paragraph, "build-depends").unwrap_or_default(),
                build_depends_indep: field_to_vec(&paragraph, "build-depends-indep")
                    .unwrap_or_default(),
                build_depends_arch: field_to_vec(&paragraph, "build-depends-arch")
                    .unwrap_or_default(),
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
    // TODO optimization: don't waste time formatting the fields of
    // packages you won't even be looking at. Leave the BinaryPackage
    // fields as their raw versions and split them later.
    let binary_packages: Vec<BinaryPackage> = paragraphs
        .into_iter()
        .filter_map(|paragraph| {
            Some(BinaryPackage {
                name: paragraph.get_single("package")?.to_string(),
                arch,
                component,
                pocket,
                depends: field_to_vec(&paragraph, "depends").unwrap_or_default(),
                pre_depends: field_to_vec(&paragraph, "pre-depends").unwrap_or_default(),
                recommends: field_to_vec(&paragraph, "recommends").unwrap_or_default(),
                suggests: field_to_vec(&paragraph, "suggests").unwrap_or_default(),
                provides: field_to_vec(&paragraph, "provides").unwrap_or_default(),
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
