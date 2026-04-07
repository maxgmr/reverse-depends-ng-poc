//! AI-generated unit tests for args.rs
#![allow(clippy::used_underscore_items)]

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
        base_args()._selected_pockets(false).unwrap(),
        Vendor::Ubuntu.pockets()
    );
}

#[test]
fn selected_pockets_devel_default_release_only() {
    assert_eq!(base_args()._selected_pockets(true).unwrap(), vec![""]);
}

#[test]
fn selected_pockets_devel_default_proposed_appends_proposed() {
    let args = Args {
        proposed: true,
        ..base_args()
    };
    assert_eq!(args._selected_pockets(true).unwrap(), vec!["", "-proposed"]);
}

#[test]
fn selected_pockets_devel_include_extra_pockets_when_filtered() {
    let args = Args {
        proposed: true,
        pockets: strings(&["-security", "updates"]),
        ..base_args()
    };
    assert_eq!(
        args._selected_pockets(true).unwrap(),
        vec!["-updates", "-security", "-proposed"]
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
    assert_eq!(args._selected_pockets(false).unwrap(), expected);
}

#[test]
fn selected_pockets_dash_prefix_accepted() {
    let args = Args {
        pockets: strings(&["-security"]),
        ..base_args()
    };
    assert_eq!(args._selected_pockets(false).unwrap(), vec!["-security"]);
}

#[test]
fn selected_pockets_no_dash_prefix_normalized() {
    // "security" and "-security" should be equivalent
    let args = Args {
        pockets: strings(&["security"]),
        ..base_args()
    };
    assert_eq!(args._selected_pockets(false).unwrap(), vec!["-security"]);
}

#[test]
fn selected_pockets_release_normalized_from_release() {
    let args = Args {
        pockets: strings(&["release"]),
        ..base_args()
    };
    assert_eq!(args._selected_pockets(false).unwrap(), vec![""]);
}

#[test]
fn selected_pockets_release_normalized_from_dash_release() {
    let args = Args {
        pockets: strings(&["-release"]),
        ..base_args()
    };
    assert_eq!(args._selected_pockets(false).unwrap(), vec![""]);
}

#[test]
fn selected_pockets_all_invalid_is_err() {
    let args = Args {
        pockets: strings(&["nonexistent"]),
        ..base_args()
    };
    assert!(args._selected_pockets(false).is_err());
}

#[test]
fn selected_pockets_proposed_flag_adds_proposed_to_filtered_result() {
    let args = Args {
        pockets: strings(&["security"]),
        proposed: true,
        ..base_args()
    };
    let pockets = args._selected_pockets(false).unwrap();
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
    let pockets = args._selected_pockets(false).unwrap();
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
        cache: true,
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
