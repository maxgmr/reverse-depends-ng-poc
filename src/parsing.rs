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

/// Parse the raw text content as a
/// [DEB822 format](https://repolib.readthedocs.io/en/latest/deb822-format.html)
/// list of source packages, returning a list of [`SourcePackage`] if
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
