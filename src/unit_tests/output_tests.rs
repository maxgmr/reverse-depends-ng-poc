//! AI-generated unit tests for output.rs

use super::*;
use crate::RevDepEntry;

// list_output tests

#[test]
fn list_output_empty_result_is_empty_string() {
    assert_eq!(list_output(&HashMap::new()), "");
}

#[test]
fn list_output_single_package() {
    let result = single_section(
        "Reverse-Depends",
        vec![entry("libfoo", "libfoo", &["amd64"])],
    );
    assert_eq!(list_output(&result), "libfoo");
}

#[test]
fn list_output_multiple_packages_sorted_alphabetically() {
    let result = single_section(
        "Reverse-Depends",
        vec![
            entry("pkg-b", "libfoo", &["amd64"]),
            entry("pkg-a", "libfoo", &["amd64"]),
        ],
    );
    assert_eq!(list_output(&result), "pkg-a\npkg-b");
}

#[test]
fn list_output_deduplicates_across_sections() {
    // pkg-a appears in both sections — must appear only once in output
    let mut result = single_section(
        "Reverse-Depends",
        vec![entry("pkg-a", "libfoo", &["amd64"])],
    );
    result.insert(
        "Reverse-Recommends",
        vec![entry("pkg-a", "libfoo", &["arm64"])],
    );
    assert_eq!(list_output(&result), "pkg-a");
}

// list_output_recursive tests

#[test]
fn list_output_recursive_empty_is_empty_string() {
    assert_eq!(list_output_recursive(&HashMap::new()), "");
}

#[test]
fn list_output_recursive_deduplicates_and_sorts_across_all_depth_levels() {
    let mut all_results: HashMap<&str, HashMap<&'static str, Vec<RevDepEntry<'_>>>> =
        HashMap::new();
    all_results.insert(
        "libfoo",
        single_section(
            "Reverse-Depends",
            vec![
                entry("pkg-c", "libfoo", &["amd64"]),
                entry("pkg-a", "libfoo", &["amd64"]),
            ],
        ),
    );
    // pkg-a appears again at a deeper level — should be deduplicated
    all_results.insert(
        "pkg-c",
        single_section(
            "Reverse-Depends",
            vec![
                entry("pkg-a", "pkg-c", &["amd64"]),
                entry("pkg-b", "pkg-c", &["amd64"]),
            ],
        ),
    );
    assert_eq!(list_output_recursive(&all_results), "pkg-a\npkg-b\npkg-c");
}

// verbose_output tests

#[test]
fn verbose_output_empty_result_is_empty_string() {
    assert_eq!(verbose_output("libfoo", &HashMap::new()), "");
}

#[test]
fn verbose_output_section_header_and_underline_format() {
    let result = single_section(
        "Reverse-Depends",
        vec![entry("pkg-a", "libfoo", &["amd64"])],
    );
    let output = verbose_output("libfoo", &result);
    assert!(output.contains("Reverse-Depends\n===============\n"));
}

#[test]
fn verbose_output_no_arch_label_when_entry_covers_all_arches() {
    let result = single_section(
        "Reverse-Depends",
        vec![entry("pkg-a", "libfoo", &["amd64", "arm64"])],
    );
    let output = verbose_output("libfoo", &result);
    // Entry covers the full arch set → no bracket label
    assert!(output.lines().any(|l| l == "* pkg-a"));
}

#[test]
fn verbose_output_arch_label_shown_when_entry_covers_subset_of_arches() {
    // Two entries with disjoint arches → each gets a label for its specific arch
    let result = single_section(
        "Reverse-Depends",
        vec![
            entry("pkg-amd64", "libfoo", &["amd64"]),
            entry("pkg-arm64", "libfoo", &["arm64"]),
        ],
    );
    let output = verbose_output("libfoo", &result);
    assert!(output.contains("* pkg-amd64 [amd64]"));
    assert!(output.contains("* pkg-arm64 [arm64]"));
}

#[test]
fn verbose_output_arch_label_arches_are_sorted() {
    // pkg-b's arches are given unsorted; a third entry makes both subsets
    let result = single_section(
        "Reverse-Depends",
        vec![
            entry("pkg-a", "libfoo", &["s390x"]),
            entry("pkg-b", "libfoo", &["arm64", "amd64"]), // intentionally unsorted
        ],
    );
    let output = verbose_output("libfoo", &result);
    assert!(output.contains("* pkg-b [amd64 arm64]"));
}

#[test]
fn verbose_output_source_arch_excluded_from_all_arches_set() {
    // source is filtered from all_arches; source-only entry_arch_set == only_source_set
    // → no arch label and no footer
    let result = single_section(
        "Reverse-Build-Depends",
        vec![entry("pkg-a", "libfoo", &["source"])],
    );
    let output = verbose_output("libfoo", &result);
    assert!(output.lines().any(|l| l == "* pkg-a"));
    assert!(!output.contains("reverse-dependencies in:"));
}

#[test]
fn verbose_output_no_annotation_when_dependency_matches_queried_package() {
    let result = single_section(
        "Reverse-Depends",
        vec![entry("pkg-a", "libfoo", &["amd64"])],
    );
    assert!(!verbose_output("libfoo", &result).contains("(for"));
}

#[test]
fn verbose_output_annotation_shown_when_dependency_differs_from_queried_package() {
    let result = single_section(
        "Reverse-Depends",
        vec![entry("pkg-a", "libfoo-dev", &["amd64"])],
    );
    assert!(verbose_output("libfoo", &result).contains("(for libfoo-dev)"));
}

#[test]
fn verbose_output_no_annotation_when_dependency_is_empty() {
    // Empty dependency differs from queried package, but is still suppressed
    let result = single_section("Reverse-Depends", vec![entry("pkg-a", "", &["amd64"])]);
    assert!(!verbose_output("libfoo", &result).contains("(for"));
}

#[test]
fn verbose_output_footer_lists_binary_arches_sorted() {
    let result = single_section(
        "Reverse-Depends",
        vec![entry("pkg-a", "libfoo", &["arm64", "amd64"])],
    );
    assert!(verbose_output("libfoo", &result).ends_with("reverse-dependencies in: amd64, arm64"));
}

#[test]
fn verbose_output_no_footer_when_no_binary_arches() {
    let result = single_section(
        "Reverse-Build-Depends",
        vec![entry("pkg-a", "libfoo", &["source"])],
    );
    assert!(!verbose_output("libfoo", &result).contains("reverse-dependencies in:"));
}

#[test]
fn verbose_output_known_sections_appear_in_field_order() {
    // HashMap iteration is unordered — ordered_fields must impose FIELD_ORDER
    let mut result = single_section(
        "Reverse-Recommends",
        vec![entry("pkg-b", "libfoo", &["amd64"])],
    );
    result.insert(
        "Reverse-Depends",
        vec![entry("pkg-a", "libfoo", &["amd64"])],
    );
    let output = verbose_output("libfoo", &result);
    assert!(output.find("Reverse-Depends") < output.find("Reverse-Recommends"));
}

#[test]
fn verbose_output_unknown_sections_appended_after_known_sections() {
    let mut result = single_section(
        "Zz-Custom-Field",
        vec![entry("pkg-b", "libfoo", &["amd64"])],
    );
    result.insert(
        "Reverse-Depends",
        vec![entry("pkg-a", "libfoo", &["amd64"])],
    );
    let output = verbose_output("libfoo", &result);
    assert!(output.find("Reverse-Depends") < output.find("Zz-Custom-Field"));
}

// verbose_output_recursive tests

#[test]
fn verbose_output_recursive_missing_root_is_empty_string() {
    let all_results: HashMap<&str, HashMap<&'static str, Vec<RevDepEntry<'_>>>> = HashMap::new();
    assert_eq!(verbose_output_recursive("libfoo", &all_results), "");
}

#[test]
#[allow(clippy::manual_contains)]
fn verbose_output_recursive_child_entries_are_indented() {
    let mut all_results: HashMap<&str, HashMap<&'static str, Vec<RevDepEntry<'_>>>> =
        HashMap::new();
    all_results.insert(
        "libfoo",
        single_section(
            "Reverse-Depends",
            vec![entry("pkg-a", "libfoo", &["amd64"])],
        ),
    );
    all_results.insert(
        "pkg-a",
        single_section("Reverse-Depends", vec![entry("pkg-b", "pkg-a", &["amd64"])]),
    );
    let output = verbose_output_recursive("libfoo", &all_results);
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines.iter().any(|&l| l == "* pkg-a")); // depth 0: no indent
    assert!(lines.iter().any(|&l| l == "  * pkg-b")); // depth 1: 2 spaces
}

#[test]
fn verbose_output_recursive_visited_set_prevents_cycle() {
    // libfoo → pkg-a → libfoo (cycle); pkg-a must appear exactly once
    let mut all_results: HashMap<&str, HashMap<&'static str, Vec<RevDepEntry<'_>>>> =
        HashMap::new();
    all_results.insert(
        "libfoo",
        single_section(
            "Reverse-Depends",
            vec![entry("pkg-a", "libfoo", &["amd64"])],
        ),
    );
    all_results.insert(
        "pkg-a",
        single_section(
            "Reverse-Depends",
            vec![entry("libfoo", "pkg-a", &["amd64"])], // points back to root
        ),
    );
    let output = verbose_output_recursive("libfoo", &all_results);
    // pkg-a rendered at depth 0, but its subtree's back-reference to libfoo
    // is blocked by visited — pkg-a must not reappear at depth 2
    assert_eq!(output.matches("* pkg-a").count(), 1);
}

// Helpers

fn entry<'a>(package: &'a str, dependency: &'a str, arches: &[&'a str]) -> RevDepEntry<'a> {
    RevDepEntry {
        package,
        architectures: arches.to_vec(),
        component: "main",
        dependency,
    }
}

fn single_section<'a>(
    field: &'static str,
    entries: Vec<RevDepEntry<'a>>,
) -> HashMap<&'static str, Vec<RevDepEntry<'a>>> {
    HashMap::from([(field, entries)])
}
