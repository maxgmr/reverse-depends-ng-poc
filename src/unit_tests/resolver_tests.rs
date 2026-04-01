//! AI-generated unit tests for resolver.rs.

use super::*;
use crate::{Args, BinaryPackage, SourcePackage, Vendor};
use std::collections::HashSet;

// source_binaries tests

#[test]
fn source_binaries_no_matching_source_returns_empty() {
    assert!(source_binaries(&[src("other", "bin-a", "")], "nonexistent").is_empty());
}

#[test]
fn source_binaries_splits_and_trims_comma_separated_names() {
    let result = source_binaries(&[src("my-src", "bin-a, bin-b,  bin-c  ", "")], "my-src");
    assert_eq!(
        result,
        HashSet::from([
            "bin-a".to_string(),
            "bin-b".to_string(),
            "bin-c".to_string()
        ])
    );
}

#[test]
fn source_binaries_merges_across_multiple_matching_source_entries() {
    // Same source name can appear in multiple pockets/components
    let sources = vec![src("my-src", "bin-a", ""), src("my-src", "bin-b", "")];
    let result = source_binaries(&sources, "my-src");
    assert_eq!(
        result,
        HashSet::from(["bin-a".to_string(), "bin-b".to_string()])
    );
}

#[test]
fn source_binaries_empty_binaries_field_returns_empty() {
    assert!(source_binaries(&[src("my-src", "", "")], "my-src").is_empty());
}

// binaries_provides tests

#[test]
fn binaries_provides_empty_target_set_returns_empty() {
    let bins = vec![BinaryPackage {
        provides: "virtual-pkg".to_string(),
        ..bin("pkg-a", "amd64", "")
    }];
    assert!(binaries_provides(&bins, &sset(&[])).is_empty());
}

#[test]
fn binaries_provides_target_with_no_provides_returns_empty() {
    assert!(binaries_provides(&[bin("pkg-a", "amd64", "")], &sset(&["pkg-a"])).is_empty());
}

#[test]
fn binaries_provides_returns_virtual_packages_of_targets() {
    let bins = vec![BinaryPackage {
        provides: "virtual-a, virtual-b".to_string(),
        ..bin("pkg-a", "amd64", "")
    }];
    assert_eq!(
        binaries_provides(&bins, &sset(&["pkg-a"])),
        HashSet::from(["virtual-a".to_string(), "virtual-b".to_string()])
    );
}

#[test]
fn binaries_provides_ignores_non_target_packages() {
    let bins = vec![
        BinaryPackage {
            provides: "virtual-a".to_string(),
            ..bin("pkg-a", "amd64", "")
        },
        BinaryPackage {
            provides: "virtual-b".to_string(),
            ..bin("pkg-b", "amd64", "")
        },
    ];
    let result = binaries_provides(&bins, &sset(&["pkg-a"])); // only pkg-a is a target
    assert!(result.contains("virtual-a"));
    assert!(!result.contains("virtual-b"));
}

// find_rev_deps binary mode tests

#[test]
fn find_rev_deps_empty_inputs_returns_empty() {
    assert!(find_rev_deps(&[], &[], &targets(&["libfoo"]), &base_args()).is_empty());
}

#[test]
fn find_rev_deps_basic_reverse_depends() {
    let bins = [bin("pkg-a", "amd64", "libfoo")];
    let result = find_rev_deps(&bins, &[], &targets(&["libfoo"]), &base_args());
    let entries = &result["Reverse-Depends"];
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].package, "pkg-a");
    assert_eq!(entries[0].dependency, "libfoo");
    assert_eq!(entries[0].architectures, vec!["amd64"]);
}

#[test]
fn find_rev_deps_non_target_dependency_not_included() {
    let bins = [bin("pkg-a", "amd64", "libbar")];
    let result = find_rev_deps(&bins, &[], &targets(&["libfoo"]), &base_args());
    assert!(result.is_empty());
}

#[test]
fn find_rev_deps_pre_depends_goes_to_reverse_pre_depends() {
    let bins = vec![BinaryPackage {
        pre_depends: "libfoo".to_string(),
        ..bin("pkg-a", "amd64", "")
    }];
    let result = find_rev_deps(&bins, &[], &targets(&["libfoo"]), &base_args());
    assert!(result.contains_key("Reverse-Pre-Depends"));
    assert!(!result.contains_key("Reverse-Depends"));
}

#[test]
fn find_rev_deps_recommends_included_when_enabled() {
    let bins = vec![BinaryPackage {
        recommends: "libfoo".to_string(),
        ..bin("pkg-a", "amd64", "")
    }];
    // recommends: true by default in base_args
    assert!(
        find_rev_deps(&bins, &[], &targets(&["libfoo"]), &base_args())
            .contains_key("Reverse-Recommends")
    );
}

#[test]
fn find_rev_deps_recommends_excluded_when_disabled() {
    let args = Args {
        recommends: false,
        ..base_args()
    };
    let bins = vec![BinaryPackage {
        recommends: "libfoo".to_string(),
        ..bin("pkg-a", "amd64", "")
    }];
    assert!(
        !find_rev_deps(&bins, &[], &targets(&["libfoo"]), &args).contains_key("Reverse-Recommends")
    );
}

#[test]
fn find_rev_deps_suggests_excluded_by_default() {
    let bins = vec![BinaryPackage {
        suggests: "libfoo".to_string(),
        ..bin("pkg-a", "amd64", "")
    }];
    assert!(
        !find_rev_deps(&bins, &[], &targets(&["libfoo"]), &base_args())
            .contains_key("Reverse-Suggests")
    );
}

#[test]
fn find_rev_deps_suggests_included_when_enabled() {
    let args = Args {
        suggests: true,
        ..base_args()
    };
    let bins = vec![BinaryPackage {
        suggests: "libfoo".to_string(),
        ..bin("pkg-a", "amd64", "")
    }];
    assert!(
        find_rev_deps(&bins, &[], &targets(&["libfoo"]), &args).contains_key("Reverse-Suggests")
    );
}

#[test]
fn find_rev_deps_or_group_including_target_stored_as_joined_expression() {
    let bins = [bin("pkg-a", "amd64", "libbar | libfoo")];
    let result = find_rev_deps(&bins, &[], &targets(&["libfoo"]), &base_args());
    assert_eq!(result["Reverse-Depends"][0].dependency, "libbar | libfoo");
}

#[test]
fn find_rev_deps_or_group_not_including_target_not_in_result() {
    let bins = [bin("pkg-a", "amd64", "libbar | libbaz")];
    let result = find_rev_deps(&bins, &[], &targets(&["libfoo"]), &base_args());
    assert!(result.is_empty());
}

#[test]
fn find_rev_deps_same_package_from_multiple_arches_merges_architectures() {
    let bins = vec![
        bin("pkg-a", "amd64", "libfoo"),
        bin("pkg-a", "arm64", "libfoo"),
    ];
    let result = find_rev_deps(&bins, &[], &targets(&["libfoo"]), &base_args());
    let entries = &result["Reverse-Depends"];
    assert_eq!(
        entries.len(),
        1,
        "pkg-a should appear as a single deduplicated entry"
    );
    assert!(entries[0].architectures.contains(&"amd64"));
    assert!(entries[0].architectures.contains(&"arm64"));
}

#[test]
fn find_rev_deps_same_package_multiple_dep_exprs_creates_separate_entries() {
    // "libfoo, libfoo | libbar" → two OR groups both matching target
    // Each distinct dep_expr is a separate accumulator key
    let bins = vec![bin("pkg-a", "amd64", "libfoo, libfoo | libbar")];
    let result = find_rev_deps(&bins, &[], &targets(&["libfoo"]), &base_args());
    assert_eq!(result["Reverse-Depends"].len(), 2);
}

#[test]
fn find_rev_deps_results_sorted_by_package_name() {
    let bins = vec![
        bin("pkg-z", "amd64", "libfoo"),
        bin("pkg-a", "amd64", "libfoo"),
        bin("pkg-m", "amd64", "libfoo"),
    ];
    let result = find_rev_deps(&bins, &[], &targets(&["libfoo"]), &base_args());
    let names: Vec<&str> = result["Reverse-Depends"]
        .iter()
        .map(|e| e.package)
        .collect();
    assert_eq!(names, vec!["pkg-a", "pkg-m", "pkg-z"]);
}

#[test]
fn find_rev_deps_binary_mode_ignores_source_packages() {
    // binary mode: want_build_depends() is false → source loop is skipped
    let sources = vec![src("src-a", "bin-a", "libfoo")];
    assert!(find_rev_deps(&[], &sources, &targets(&["libfoo"]), &base_args()).is_empty());
}

// find_rev_deps build mode tests

#[test]
fn find_rev_deps_build_mode_ignores_binary_packages() {
    // build mode: want_build_depends() is true → binary loop is skipped
    assert!(
        find_rev_deps(
            &[bin("pkg-a", "amd64", "libfoo")],
            &[],
            &targets(&["libfoo"]),
            &build_args()
        )
        .is_empty()
    );
}

#[test]
fn find_rev_deps_build_depends_goes_to_reverse_build_depends_with_source_arch() {
    let srcs = [src("src-a", "bin-a", "libfoo")];
    let result = find_rev_deps(&[], &srcs, &targets(&["libfoo"]), &build_args());
    let entries = &result["Reverse-Build-Depends"];
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].package, "src-a");
    assert_eq!(entries[0].architectures, vec!["source"]);
}

#[test]
fn find_rev_deps_build_depends_indep_goes_to_correct_group() {
    let sources = vec![SourcePackage {
        build_depends_indep: "libfoo".to_string(),
        ..src("src-a", "bin-a", "")
    }];
    let result = find_rev_deps(&[], &sources, &targets(&["libfoo"]), &build_args());
    assert!(result.contains_key("Reverse-Build-Depends-Indep"));
    assert!(!result.contains_key("Reverse-Build-Depends"));
}

#[test]
fn find_rev_deps_build_depends_arch_goes_to_correct_group() {
    let sources = vec![SourcePackage {
        build_depends_arch: "libfoo".to_string(),
        ..src("src-a", "bin-a", "")
    }];
    let result = find_rev_deps(&[], &sources, &targets(&["libfoo"]), &build_args());
    assert!(result.contains_key("Reverse-Build-Depends-Arch"));
    assert!(!result.contains_key("Reverse-Build-Depends"));
}

#[test]
fn find_rev_deps_testsuite_triggers_direct_match() {
    let sources = vec![SourcePackage {
        testsuite_triggers: "libfoo, other-pkg".to_string(),
        ..src("src-a", "bin-a", "")
    }];
    let result = find_rev_deps(&[], &sources, &targets(&["libfoo"]), &build_args());
    let entries = &result["Reverse-Testsuite-Triggers"];
    assert_eq!(entries[0].package, "src-a");
    assert_eq!(entries[0].dependency, ""); // empty dep string for testsuite triggers
}

#[test]
fn find_rev_deps_testsuite_builddeps_expands_to_build_dep_match() {
    // @builddeps@ means "re-run tests if any build dep changes"
    let sources = vec![SourcePackage {
        build_depends: "libfoo (>= 1.0)".to_string(),
        testsuite_triggers: "@builddeps@".to_string(),
        ..src("src-a", "bin-a", "")
    }];
    let result = find_rev_deps(&[], &sources, &targets(&["libfoo"]), &build_args());
    assert!(result.contains_key("Reverse-Testsuite-Triggers"));
}

#[test]
fn find_rev_deps_testsuite_builddeps_no_match_not_in_result() {
    let sources = vec![SourcePackage {
        build_depends: "libbar".to_string(), // doesn't match target
        testsuite_triggers: "@builddeps@".to_string(),
        ..src("src-a", "bin-a", "")
    }];
    assert!(
        !find_rev_deps(&[], &sources, &targets(&["libfoo"]), &build_args())
            .contains_key("Reverse-Testsuite-Triggers")
    );
}

#[test]
fn find_rev_deps_testsuite_direct_and_builddeps_match_creates_single_entry() {
    // Both conditions fire for the same source — must not duplicate the entry
    let sources = vec![SourcePackage {
        build_depends: "libfoo".to_string(),
        testsuite_triggers: "libfoo, @builddeps@".to_string(),
        ..src("src-a", "bin-a", "")
    }];
    let result = find_rev_deps(&[], &sources, &targets(&["libfoo"]), &build_args());
    assert_eq!(result["Reverse-Testsuite-Triggers"].len(), 1);
}

// find_dev_deps_recursive

#[test]
fn find_rev_deps_recursive_root_always_present_even_with_no_results() {
    let result = find_rev_deps_recursive(&[], &[], "libfoo", &targets(&["libfoo"]), &base_args());
    assert!(result.contains_key("libfoo"));
    assert!(result["libfoo"].is_empty());
}

#[test]
fn find_rev_deps_recursive_one_level_chain() {
    let bins = vec![
        bin("pkg-a", "amd64", "libfoo"),
        bin("pkg-b", "amd64", "pkg-a"),
    ];
    let result = find_rev_deps_recursive(&bins, &[], "libfoo", &targets(&["libfoo"]), &base_args());
    assert!(
        result["libfoo"]["Reverse-Depends"]
            .iter()
            .any(|e| e.package == "pkg-a")
    );
    assert!(
        result["pkg-a"]["Reverse-Depends"]
            .iter()
            .any(|e| e.package == "pkg-b")
    );
}

#[test]
fn find_rev_deps_recursive_depth_limit_stops_traversal() {
    // Chain: libfoo ← pkg-a ← pkg-b ← pkg-c; depth=1 should stop after pkg-a
    let args = Args {
        recursive_depth: 1,
        ..base_args()
    };
    let bins = vec![
        bin("pkg-a", "amd64", "libfoo"),
        bin("pkg-b", "amd64", "pkg-a"),
        bin("pkg-c", "amd64", "pkg-b"),
    ];
    let result = find_rev_deps_recursive(&bins, &[], "libfoo", &targets(&["libfoo"]), &args);
    assert!(result.contains_key("libfoo"));
    assert!(result.contains_key("pkg-a")); // depth 1: pkg-a's rev deps processed
    assert!(!result.contains_key("pkg-b")); // depth 2: not reached
}

#[test]
fn find_rev_deps_recursive_package_with_no_rev_deps_omitted_from_results() {
    // pkg-a depends on libfoo but nothing depends on pkg-a
    let bins = vec![bin("pkg-a", "amd64", "libfoo")];
    let result = find_rev_deps_recursive(&bins, &[], "libfoo", &targets(&["libfoo"]), &base_args());
    assert!(result.contains_key("libfoo")); // root always inserted
    assert!(!result.contains_key("pkg-a")); // no rev deps → omitted
}

// Helpers

fn base_args() -> Args {
    Args {
        package: "libfoo".to_string(),
        release: None,
        vendor: Vendor::Ubuntu,
        recommends: true,
        suggests: false,
        provides: false,
        build_depends: false,
        arches: vec!["any".to_string()],
        ports: true,
        components: vec![],
        pockets: vec![],
        proposed: false,
        list: false,
        recursive: false,
        recursive_depth: 10,
    }
}

fn build_args() -> Args {
    Args {
        build_depends: true,
        ..base_args()
    }
}

fn bin(name: &str, arch: &'static str, depends: &str) -> BinaryPackage {
    BinaryPackage {
        name: name.to_string(),
        arch,
        component: "main",
        pocket: "",
        depends: depends.to_string(),
        pre_depends: String::new(),
        recommends: String::new(),
        suggests: String::new(),
        provides: String::new(),
    }
}

fn src(name: &str, binaries: &str, build_depends: &str) -> SourcePackage {
    SourcePackage {
        name: name.to_string(),
        component: "main",
        pocket: "",
        binaries: binaries.to_string(),
        build_depends: build_depends.to_string(),
        build_depends_indep: String::new(),
        build_depends_arch: String::new(),
        testsuite_triggers: String::new(),
    }
}

/// `HashSet<String>` for `binaries_provides` (which takes `&HashSet<String>`).
fn sset(names: &[&str]) -> HashSet<String> {
    names.iter().map(ToString::to_string).collect()
}

/// `HashSet<&str>` for `find_rev_deps` target sets.
fn targets<'a>(names: &[&'a str]) -> HashSet<&'a str> {
    names.iter().copied().collect()
}
