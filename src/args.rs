//! This module contains CLI argument handling.

use std::collections::HashSet;

use crate::Vendor;

use clap::Parser;

const ARCH_DEFAULT: &str = "any";
const DEFAULT_DEPTH: usize = 10;

// TODO potential optimization: add "cached" option which allows the
// usage of cached data
#[allow(clippy::doc_markdown)]
/// List reverse-dependencies of an Ubuntu/Debian package.
///
/// If PACKAGE is prefixed with `src:`, the reverse-dependencies of all
/// binary packages produced by that source package will be listed.
///
/// Unlike the original `reverse-depends`, this tool queries the Ubuntu
/// archive directly rather than relying on the UbuntuWire web service.
/// This correctly handles virtual packages, `Provides:` declarations,
/// and the Rust crate ecosystem's use of `Provides` in build
/// dependencies.
#[derive(Parser, Debug)]
#[command(name = "reverse-depends", author, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
pub struct Args {
    /// Package to query (prefix with `src:` for source packages)
    pub package: String,
    /// Query dependencies in RELEASE [default: current devel release]
    #[arg(short, long)]
    pub release: Option<String>,
    /// Distro to check
    #[arg(short = 'V', long, value_enum, default_value_t = Vendor::Ubuntu)]
    pub vendor: Vendor,
    /// Ignore Recommends relationships
    #[arg(short = 'R', long = "without-recommends", action = clap::ArgAction::SetFalse)]
    pub recommends: bool,
    /// Also consider Suggests relationships
    #[arg(short = 's', long = "with-suggests")]
    pub suggests: bool,
    /// Also consider Provides relationships
    #[arg(short = 'p', long = "with-provides")]
    pub provides: bool,
    /// Query build dependencies instead of binary dependencies;
    /// equivalent to `--arch=source`
    #[arg(short, long = "build-depends")]
    pub build_depends: bool,
    /// Query dependencies in ARCH, or `source` for build dependencies
    /// (repeatable)
    #[arg(short, long, default_value = ARCH_DEFAULT)]
    pub arches: Vec<String>,
    /// Skip ports architectures.
    #[arg(long = "no-ports", action = clap::ArgAction::SetFalse)]
    pub ports: bool,
    /// Only consider reverse dependencies in COMPONENT (repeatable)
    #[arg(short, long = "component")]
    pub components: Vec<String>,
    // TODO add proposed argument
    // TODO add "only consider reverse dependencies in POCKET (repeatable)"
    /// Display a simple, machine-readable list
    #[arg(short, long)]
    pub list: bool,
    /// Find reverse dependencies recursively
    #[arg(short = 'x', long)]
    pub recursive: bool,
    /// Maximum depth of recursion when `--recursive` is set
    #[arg(
        short = 'd',
        long = "recursive-depth",
        value_name = "DEPTH",
        default_value_t = DEFAULT_DEPTH
    )]
    pub recursive_depth: usize,
}
impl Args {
    /// Get the list of components in the selected [`Vendor`] which
    /// were selected by [`Args::components`].
    ///
    /// If [`Args::components`] is empty, then all components in the
    /// selected [`Vendor`] are selected.
    ///
    /// # Errors
    ///
    /// This function returns an [`anyhow::Error`] if the returned list is
    /// empty.
    pub fn selected_components(&self) -> anyhow::Result<Vec<&'static str>> {
        #[allow(clippy::used_underscore_items)]
        _selected_components(self.vendor.components(), &self.components)
    }

    /// Returns `true` if and only if fetching source packages is
    /// required.
    #[must_use]
    pub fn need_source_packages(&self) -> bool {
        self.want_build_depends() || self.package.starts_with("src:")
    }

    /// Returns `true` if and only if the program should consider build
    /// dependencies.
    #[must_use]
    pub fn want_build_depends(&self) -> bool {
        self.build_depends || self.arches.iter().any(|s| s == "source")
    }

    /// Returns the set of [`ArchSearchCombo`]s to query for the
    /// chosen list of architectures for the given release.
    #[must_use]
    pub fn needed_arch_searches(&self, release: &str) -> HashSet<ArchSearchCombo> {
        let mut combos = HashSet::new();

        for arch in &self.arches {
            match arch.as_str() {
                "any" => {
                    // Search for all primary and ports arches
                    for arch in self.vendor.primary_arches() {
                        combos.insert(ArchSearchCombo::new(self.vendor.archive(), arch));
                    }
                    if self.ports {
                        for arch in self.vendor.ports_arches(release) {
                            combos.insert(ArchSearchCombo::new(self.vendor.ports(), arch));
                        }
                    }
                }
                // No binary package lists to search for source
                "source" => (),
                a => {
                    // Route the arch to whichever archive carries it
                    if self.ports
                        && let Some(&arch) =
                            self.vendor.ports_arches(release).iter().find(|&&s| s == a)
                    {
                        combos.insert(ArchSearchCombo::new(self.vendor.ports(), arch));
                    } else if let Some(&arch) =
                        self.vendor.primary_arches().iter().find(|&&s| s == a)
                    {
                        combos.insert(ArchSearchCombo::new(self.vendor.archive(), arch));
                    }
                    // Unknown arch: skip
                }
            }
        }

        combos
    }
}

/// A specific set of values to use in a binary package search: the
/// base archive URL of the search and the associated architecture.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArchSearchCombo {
    /// The base archive URL of the search.
    pub base_url: &'static str,
    /// The architecture of the search.
    pub arch: &'static str,
}
impl ArchSearchCombo {
    fn new(base_url: &'static str, arch: &'static str) -> Self {
        Self { base_url, arch }
    }
}

/// Helper to make testing easier
fn _selected_components(
    vendor_components: &'static [&'static str],
    arg_components: &[String],
) -> anyhow::Result<Vec<&'static str>> {
    if arg_components.is_empty() {
        return Ok(vendor_components.to_vec());
    }

    let result: Vec<&'static str> = vendor_components
        .iter()
        .copied()
        .filter(|&vendor_component| {
            arg_components
                .iter()
                .any(|arg_component| arg_component.as_str() == vendor_component)
        })
        .collect();

    if result.is_empty() {
        anyhow::bail!("No components named {arg_components:?} exist");
    }

    Ok(result)
}

#[cfg(test)]
#[allow(clippy::used_underscore_items)]
mod tests {
    use super::*;

    #[test]
    fn all_ubuntu_components() {
        assert_eq!(
            _selected_components(Vendor::Ubuntu.components(), &[]).unwrap(),
            Vendor::Ubuntu.components()
        );
    }

    #[test]
    fn all_debian_components() {
        assert_eq!(
            _selected_components(Vendor::Debian.components(), &[]).unwrap(),
            Vendor::Debian.components()
        );
    }

    #[test]
    fn invalid_components() {
        _selected_components(
            Vendor::Ubuntu.components(),
            &[String::from("nonexistent component")],
        )
        .unwrap_err();
    }

    #[test]
    fn some_ubuntu_components() {
        assert_eq!(
            _selected_components(
                Vendor::Ubuntu.components(),
                &[String::from("main"), String::from("restricted")]
            )
            .unwrap(),
            vec!["main", "restricted"]
        );
    }

    #[test]
    fn one_valid_component() {
        assert_eq!(
            _selected_components(
                Vendor::Debian.components(),
                &[
                    String::from("nonexistent component"),
                    String::from("non-free-firmware")
                ]
            )
            .unwrap(),
            vec!["non-free-firmware"]
        );
    }
}
