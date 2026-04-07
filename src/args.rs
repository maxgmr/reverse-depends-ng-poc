//! This module contains CLI argument handling.

use std::collections::HashSet;

use crate::{Vendor, detect_devel_release};

use clap::Parser;

const ARCH_DEFAULT: &str = "any";
const DEFAULT_DEPTH: usize = 10;

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
    /// Skip ports architectures
    #[arg(long = "no-ports", action = clap::ArgAction::SetFalse)]
    pub ports: bool,
    /// Only consider reverse dependencies in COMPONENT (repeatable)
    #[arg(short, long = "component", value_name = "COMPONENT")]
    pub components: Vec<String>,
    /// Only consider reverse dependencies in POCKET (repeatable)
    #[arg(short = 'k', long = "pocket", value_name = "POCKET")]
    pub pockets: Vec<String>,
    /// Also consider proposed pocket
    #[arg(long)]
    pub proposed: bool,
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
    /// Avoid using any archive caches, querying new archive data no
    /// matter what
    #[arg(short = 'C', long = "no-cache", action = clap::ArgAction::SetFalse)]
    pub cache: bool,
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
    /// This function returns an [`anyhow::Error`] if none of the
    /// provided components exist in the archive.
    pub fn selected_components(&self) -> anyhow::Result<Vec<&'static str>> {
        if self.components.is_empty() {
            return Ok(self.vendor.components().to_vec());
        }

        let result: Vec<&'static str> = self
            .vendor
            .components()
            .iter()
            .copied()
            .filter(|&known_val| {
                self.components
                    .iter()
                    .any(|given_val| given_val.as_str() == known_val)
            })
            .collect();

        if result.is_empty() {
            anyhow::bail!("No components named {:?} exist", self.components);
        }

        Ok(result)
    }

    /// Get the list of pockets in the selected [`Vendor`] which were
    /// selected by [`Args::pockets`].
    ///
    /// If [`Args::pockets`] is empty, then all pockets except
    /// `proposed` are selected, unless it's the current devel release,
    /// in which case only the `release` pocket is selected.
    ///
    /// # Errors
    ///
    /// This function returns an [`anyhow::Error`] if none of the
    /// provided pockets exist in the archive, or if there is a failure
    /// when trying to detect the current devel release.
    pub fn selected_pockets(&self) -> anyhow::Result<Vec<&'static str>> {
        let is_devel = self.release.is_none() || self.release == Some(detect_devel_release()?);
        #[allow(clippy::used_underscore_items)]
        self._selected_pockets(is_devel)
    }

    /// Helper function for [`Self::selected_pockets`] to make unit
    /// tests possible without querying the devel release.
    fn _selected_pockets(&self, is_devel: bool) -> anyhow::Result<Vec<&'static str>> {
        // Select all pockets except proposed if no pocket args given,
        // also enabling proposed if --proposed is given
        //
        // If the given release is the current devel release, skip the
        // pockets which only apply to post-devel releases
        if self.pockets.is_empty() {
            let mut pockets = if is_devel {
                vec![""]
            } else {
                self.vendor.pockets().to_vec()
            };
            if self.proposed {
                pockets.push("-proposed");
            }
            return Ok(pockets);
        }

        // Since we're filtering pockets with `--pocket` args, it's OK
        // to add the "-proposed" pocket to the list of available
        // pockets.
        let all_pockets: Vec<&'static str> = self
            .vendor
            .pockets()
            .iter()
            .copied()
            .chain(std::iter::once("-proposed"))
            .collect();

        // Normalize args to URL format: "release" -> ""; add "-" if
        // missing
        let normalized: Vec<String> = self
            .pockets
            .iter()
            .map(|p| {
                if p == "release" || p == "-release" {
                    String::new()
                } else if p.starts_with('-') {
                    p.clone()
                } else {
                    format!("-{p}")
                }
            })
            .collect();

        let mut result: Vec<&'static str> = all_pockets
            .iter()
            .copied()
            .filter(|&known| normalized.iter().any(|n| n.as_str() == known))
            .collect();

        // Add proposed if the proposed flag is given but it wasn't
        // listed as a --pocket filter
        if self.proposed && !result.contains(&"-proposed") {
            result.push("-proposed");
        }

        if result.is_empty() {
            anyhow::bail!("No pockets named {:?} exist", self.pockets);
        }

        Ok(result)
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

// AI-generated unit tests
#[cfg(test)]
#[path = "unit_tests/args_tests.rs"]
mod tests;
