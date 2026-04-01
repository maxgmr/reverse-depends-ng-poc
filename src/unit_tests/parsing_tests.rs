//! AI-generated unit tests for parsing.rs

use super::*;

// extract_name tests

#[test]
fn extract_name_empty_string_is_none() {
    assert_eq!(extract_name(""), None);
}

#[test]
fn extract_name_whitespace_only_is_none() {
    assert_eq!(extract_name("   "), None);
}

#[test]
fn extract_name_plain_name() {
    assert_eq!(extract_name("libc6"), Some("libc6"));
}

#[test]
fn extract_name_strips_surrounding_whitespace() {
    assert_eq!(extract_name("  libc6  "), Some("libc6"));
}

#[test]
fn extract_name_strips_version_constraint() {
    assert_eq!(extract_name("libc6 (>= 2.17)"), Some("libc6"));
}

#[test]
fn extract_name_strips_arch_restriction() {
    assert_eq!(extract_name("libfoo [amd64]"), Some("libfoo"));
}

#[test]
fn extract_name_strips_multiarch_qualifier() {
    assert_eq!(extract_name("libfoo:amd64"), Some("libfoo"));
}

#[test]
fn extract_name_strips_version_and_arch_together() {
    assert_eq!(extract_name("libc6 (>= 2.17) [amd64]"), Some("libc6"));
}

// parse_provides tests

#[test]
fn parse_provides_empty_string_returns_empty() {
    assert!(parse_provides("").is_empty());
}

#[test]
fn parse_provides_single_package() {
    assert_eq!(parse_provides("virtual-pkg"), vec!["virtual-pkg"]);
}

#[test]
fn parse_provides_multiple_packages() {
    assert_eq!(parse_provides("pkg-a, pkg-b"), vec!["pkg-a", "pkg-b"]);
}

#[test]
fn parse_provides_strips_version_constraints() {
    assert_eq!(
        parse_provides("pkg-a (= 1.0), pkg-b"),
        vec!["pkg-a", "pkg-b"]
    );
}

#[test]
fn parse_provides_filters_empty_entries() {
    // A doubled comma produces a whitespace-only token that gets filtered
    assert_eq!(parse_provides("pkg-a, , pkg-b"), vec!["pkg-a", "pkg-b"]);
}

// parse_dep_names tests

#[test]
fn parse_dep_names_empty_string_returns_empty() {
    assert!(parse_dep_names("").is_empty());
}

#[test]
fn parse_dep_names_single_dep() {
    assert_eq!(parse_dep_names("libc6"), vec![vec!["libc6"]]);
}

#[test]
fn parse_dep_names_and_group() {
    assert_eq!(
        parse_dep_names("libc6, libfoo"),
        vec![vec!["libc6"], vec!["libfoo"]]
    );
}

#[test]
fn parse_dep_names_or_group() {
    assert_eq!(
        parse_dep_names("libfoo | libbar"),
        vec![vec!["libfoo", "libbar"]]
    );
}

#[test]
fn parse_dep_names_mixed_and_and_or() {
    assert_eq!(
        parse_dep_names("libc6, libfoo | libbar"),
        vec![vec!["libc6"], vec!["libfoo", "libbar"]]
    );
}

#[test]
fn parse_dep_names_strips_version_constraints() {
    assert_eq!(parse_dep_names("libc6 (>= 2.17)"), vec![vec!["libc6"]]);
}

#[test]
fn parse_dep_names_strips_arch_restrictions() {
    assert_eq!(parse_dep_names("libfoo [amd64]"), vec![vec!["libfoo"]]);
}

#[test]
fn parse_dep_names_trailing_comma_filtered() {
    // Trailing comma → empty AND group → filtered
    assert_eq!(parse_dep_names("libc6,"), vec![vec!["libc6"]]);
}

#[test]
fn parse_dep_names_trailing_pipe_filtered() {
    // Trailing pipe → whitespace-only OR entry → filtered, group kept if non-empty
    assert_eq!(parse_dep_names("libc6 | "), vec![vec!["libc6"]]);
}
