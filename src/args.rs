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
    /// `proposed` are selected.
    ///
    /// # Errors
    ///
    /// This function returns an [`anyhow::Error`] if none of the
    /// provided pockets exist in the archive.
    pub fn selected_pockets(&self) -> anyhow::Result<Vec<&'static str>> {
        // Select all pockets except proposed if no pocket args given,
        // also enabling proposed if --proposed is given
        if self.pockets.is_empty() {
            let mut pockets = self.vendor.pockets().to_vec();
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
mod tests {
    use super::*;
    use crate::Vendor;
    use std::collections::HashSet;

    #[test]
    fn selected_components_empty_returns_all_ubuntu() {
        assert_eq!(
            base_args().selected_components().unwrap(),
            Vendor::Ubuntu.components()
        );
    }

    #[test]
    fn selected_components_empty_returns_all_debian() {
        let args = Args {
            vendor: Vendor::Debian,
            ..base_args()
        };
        assert_eq!(
            args.selected_components().unwrap(),
            Vendor::Debian.components()
        );
    }

    #[test]
    fn selected_components_single_valid() {
        let args = Args {
            components: strings(&["main"]),
            ..base_args()
        };
        assert_eq!(args.selected_components().unwrap(), vec!["main"]);
    }

    #[test]
    fn selected_components_multiple_valid_returned_in_vendor_order() {
        // Vendor order for Ubuntu: main, restricted, universe, multiverse
        let args = Args {
            components: strings(&["universe", "main"]),
            ..base_args()
        };
        assert_eq!(
            args.selected_components().unwrap(),
            vec!["main", "universe"]
        );
    }

    #[test]
    fn selected_components_all_invalid_is_err() {
        let args = Args {
            components: strings(&["nonexistent"]),
            ..base_args()
        };
        assert!(args.selected_components().is_err());
    }

    #[test]
    fn selected_components_mixed_valid_and_invalid_returns_valid_only() {
        // Unknown names are silently dropped; Ok as long as ≥1 valid component remains
        let args = Args {
            components: strings(&["main", "nonexistent"]),
            ..base_args()
        };
        assert_eq!(args.selected_components().unwrap(), vec!["main"]);
    }

    #[test]
    fn selected_pockets_empty_excludes_proposed() {
        assert_eq!(
            base_args().selected_pockets().unwrap(),
            Vendor::Ubuntu.pockets()
        );
    }

    #[test]
    fn selected_pockets_empty_proposed_flag_appends_proposed() {
        let args = Args {
            proposed: true,
            ..base_args()
        };
        let mut expected = Vendor::Ubuntu.pockets().to_vec();
        expected.push("-proposed");
        assert_eq!(args.selected_pockets().unwrap(), expected);
    }

    #[test]
    fn selected_pockets_dash_prefix_accepted() {
        let args = Args {
            pockets: strings(&["-security"]),
            ..base_args()
        };
        assert_eq!(args.selected_pockets().unwrap(), vec!["-security"]);
    }

    #[test]
    fn selected_pockets_no_dash_prefix_normalized() {
        // "security" and "-security" should be equivalent
        let args = Args {
            pockets: strings(&["security"]),
            ..base_args()
        };
        assert_eq!(args.selected_pockets().unwrap(), vec!["-security"]);
    }

    #[test]
    fn selected_pockets_release_normalized_from_release() {
        let args = Args {
            pockets: strings(&["release"]),
            ..base_args()
        };
        assert_eq!(args.selected_pockets().unwrap(), vec![""]);
    }

    #[test]
    fn selected_pockets_release_normalized_from_dash_release() {
        let args = Args {
            pockets: strings(&["-release"]),
            ..base_args()
        };
        assert_eq!(args.selected_pockets().unwrap(), vec![""]);
    }

    #[test]
    fn selected_pockets_all_invalid_is_err() {
        let args = Args {
            pockets: strings(&["nonexistent"]),
            ..base_args()
        };
        assert!(args.selected_pockets().is_err());
    }

    #[test]
    fn selected_pockets_proposed_flag_adds_proposed_to_filtered_result() {
        let args = Args {
            pockets: strings(&["security"]),
            proposed: true,
            ..base_args()
        };
        let pockets = args.selected_pockets().unwrap();
        assert!(pockets.contains(&"-security"));
        assert!(pockets.contains(&"-proposed"));
    }

    #[test]
    fn selected_pockets_proposed_flag_no_duplicate_when_pocket_already_listed() {
        let args = Args {
            pockets: strings(&["proposed"]),
            proposed: true,
            ..base_args()
        };
        let pockets = args.selected_pockets().unwrap();
        let count = pockets.iter().filter(|&&p| p == "-proposed").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn want_build_depends_false_by_default() {
        assert!(!base_args().want_build_depends());
    }

    #[test]
    fn want_build_depends_true_via_build_depends_flag() {
        let args = Args {
            build_depends: true,
            ..base_args()
        };
        assert!(args.want_build_depends());
    }

    #[test]
    fn want_build_depends_true_via_source_in_arches() {
        let args = Args {
            arches: strings(&["source"]),
            ..base_args()
        };
        assert!(args.want_build_depends());
    }

    #[test]
    fn need_source_packages_false_by_default() {
        assert!(!base_args().need_source_packages());
    }

    #[test]
    fn need_source_packages_true_for_src_prefix() {
        let args = Args {
            package: "src:mypkg".to_string(),
            ..base_args()
        };
        assert!(args.need_source_packages());
    }

    #[test]
    fn need_source_packages_true_when_want_build_depends() {
        // build_depends subsumes need_source_packages
        let args = Args {
            build_depends: true,
            ..base_args()
        };
        assert!(args.need_source_packages());
    }

    const RELEASE: &str = "noble";
    const UBUNTU_ARCHIVE: &str = "http://archive.ubuntu.com/ubuntu";
    const UBUNTU_PORTS: &str = "http://ports.ubuntu.com/ubuntu-ports";

    #[test]
    fn needed_arch_searches_any_without_ports_returns_only_primary_arches() {
        let args = Args {
            ports: false,
            ..base_args()
        };
        let expected = combos(&[
            (UBUNTU_ARCHIVE, "amd64"),
            (UBUNTU_ARCHIVE, "amd64v3"),
            (UBUNTU_ARCHIVE, "i386"),
        ]);
        assert_eq!(args.needed_arch_searches(RELEASE), expected);
    }

    #[test]
    fn needed_arch_searches_any_with_ports_returns_primary_and_ports_arches() {
        let args = base_args(); // ports: true
        let mut expected = HashSet::new();
        for &arch in Vendor::Ubuntu.primary_arches() {
            expected.insert(ArchSearchCombo {
                base_url: UBUNTU_ARCHIVE,
                arch,
            });
        }
        for &arch in Vendor::Ubuntu.ports_arches(RELEASE) {
            expected.insert(ArchSearchCombo {
                base_url: UBUNTU_PORTS,
                arch,
            });
        }
        assert_eq!(args.needed_arch_searches(RELEASE), expected);
    }

    #[test]
    fn needed_arch_searches_source_arch_returns_empty() {
        let args = Args {
            arches: strings(&["source"]),
            ..base_args()
        };
        assert!(args.needed_arch_searches(RELEASE).is_empty());
    }

    #[test]
    fn needed_arch_searches_named_primary_arch() {
        let args = Args {
            arches: strings(&["amd64"]),
            ..base_args()
        };
        assert_eq!(
            args.needed_arch_searches(RELEASE),
            combos(&[(UBUNTU_ARCHIVE, "amd64")])
        );
    }

    #[test]
    fn needed_arch_searches_named_ports_arch_with_ports_enabled() {
        let args = Args {
            arches: strings(&["arm64"]),
            ports: true,
            ..base_args()
        };
        assert_eq!(
            args.needed_arch_searches(RELEASE),
            combos(&[(UBUNTU_PORTS, "arm64")])
        );
    }

    #[test]
    fn needed_arch_searches_named_ports_arch_with_ports_disabled_returns_empty() {
        let args = Args {
            arches: strings(&["arm64"]),
            ports: false,
            ..base_args()
        };
        assert!(args.needed_arch_searches(RELEASE).is_empty());
    }

    #[test]
    fn needed_arch_searches_unknown_arch_returns_empty() {
        let args = Args {
            arches: strings(&["not-an-arch"]),
            ..base_args()
        };
        assert!(args.needed_arch_searches(RELEASE).is_empty());
    }

    #[test]
    fn needed_arch_searches_mixed_primary_and_ports_arches() {
        let args = Args {
            arches: strings(&["amd64", "arm64"]),
            ports: true,
            ..base_args()
        };
        let expected = combos(&[(UBUNTU_ARCHIVE, "amd64"), (UBUNTU_PORTS, "arm64")]);
        assert_eq!(args.needed_arch_searches(RELEASE), expected);
    }

    // Helpers

    /// Minimal, valid [`Args`] with sensible defaults to be used for
    /// testing. Tests only need to override what they care about.
    fn base_args() -> Args {
        Args {
            package: "mypkg".to_string(),
            release: None,
            vendor: Vendor::Ubuntu,
            recommends: true,
            suggests: false,
            provides: false,
            build_depends: false,
            arches: vec![ARCH_DEFAULT.to_string()],
            ports: true,
            components: Vec::new(),
            pockets: Vec::new(),
            proposed: false,
            list: false,
            recursive: false,
            recursive_depth: DEFAULT_DEPTH,
        }
    }

    fn strings(strs: &[&str]) -> Vec<String> {
        strs.iter().map(ToString::to_string).collect()
    }

    fn combos(pairs: &[(&'static str, &'static str)]) -> HashSet<ArchSearchCombo> {
        pairs
            .iter()
            .map(|&(base_url, arch)| ArchSearchCombo { base_url, arch })
            .collect()
    }
}
