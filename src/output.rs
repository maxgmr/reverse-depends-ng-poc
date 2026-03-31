//! This module contains all functionality responsible for formatting
//! the output.

use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

use crate::RevDepEntry;

/// Display order for the original reverse-depends tool. Any group not
/// listed here is shown last.
const FIELD_ORDER: &[&str] = &[
    "Reverse-Depends",
    "Reverse-Recommends",
    "Reverse-Suggests",
    "Reverse-Build-Depends",
    "Reverse-Build-Depends-Indep",
    "Reverse-Build-Depends-Arch",
];

/// Original reverse-depends column padding
const PADDING: usize = 30;

/// Get the formatted output string.
///
/// Example:
/// ```text
/// Reverse-Depends
/// ===============
/// * libfoo-dev [amd64 i386]   (for libbar-dev | libbar1.1-dev)
/// * python3-baz
///
/// Reverse-Recommends
/// ==================
/// * rust-quux
///
/// Packages without architectures listed are reverse-dependencies in: amd64, arm64, i386, ...
/// ```
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn verbose_output(
    queried_package: &str,
    result: &HashMap<&'static str, Vec<RevDepEntry<'_>>>,
) -> String {
    let mut output = String::new();
    let all_arches: HashSet<&'static str> = result
        .values()
        .flat_map(|entries| entries.iter())
        .flat_map(|e| e.architectures.iter().copied())
        .filter(|&a| a != "source")
        .collect();
    let ordered = ordered_fields(result);

    for field in ordered {
        let entries = &result[field];

        // Add section header
        output.push_str(field);
        output.push('\n');
        output.push_str(&"=".repeat(field.len()));
        output.push('\n');

        // Add entries
        for entry in entries {
            output.push_str(&format_entry(entry, &all_arches, queried_package));
            output.push('\n');
        }

        // Blank line between sections
        output.push('\n');
    }

    // Footer: denote which architectures the unlabelled packages are
    // reverse dependencies on.
    if !all_arches.is_empty() {
        let mut sorted: Vec<&str> = all_arches.iter().copied().collect();
        sorted.sort_unstable();
        let _ = write!(
            &mut output,
            "Packages without architectures listed are reverse-dependencies in: {}",
            sorted.join(", ")
        );
    }

    output
}

/// Return the result's field names in the original display order.
///
/// Any fields missing from [`FIELD_ORDER`] are appended alphabetically.
fn ordered_fields(result: &HashMap<&'static str, Vec<RevDepEntry<'_>>>) -> Vec<&'static str> {
    let mut ordered: Vec<&'static str> = FIELD_ORDER
        .iter()
        .copied()
        .filter(|&f| result.contains_key(f))
        .collect();

    let mut extras: Vec<&'static str> = result
        .keys()
        .copied()
        .filter(|k| !FIELD_ORDER.contains(k))
        .collect();

    extras.sort_unstable();

    ordered.extend(extras);
    ordered
}

/// Format a single reverse dependency entry as a string.
///
/// Architecture labels are shown only when the entry does not cover all
/// of the architectures present in the full result set.
///
/// A `(for <dep>)` annotation is added when the matched dependency
/// differs from the bare queried package name.
fn format_entry(
    entry: &RevDepEntry<'_>,
    all_arches: &HashSet<&'static str>,
    queried_package: &str,
) -> String {
    let arch_label = {
        let entry_arch_set: HashSet<_> = entry.architectures.iter().collect();
        let all_arch_set: HashSet<_> = all_arches.iter().collect();
        let only_source_set = HashSet::from([&"source"]);
        if entry_arch_set == all_arch_set || entry_arch_set == only_source_set {
            // No need to print a label if the package is source only or
            // contains all architectures.
            String::new()
        } else {
            let mut sorted: Vec<_> = entry.architectures.clone();
            sorted.sort_unstable();
            format!(" [{}]", sorted.join(" "))
        }
    };

    let lhs = format!("* {}{}", entry.package, arch_label);

    let annotation = if entry.dependency != queried_package && !entry.dependency.is_empty() {
        format!("  (for {})", entry.dependency)
    } else {
        String::new()
    };

    if annotation.is_empty() {
        lhs
    } else {
        // Right-pad `lhs` to column 30 before the annotation, matching
        // the original reverse-depends behaviour.
        let padding = if lhs.len() < PADDING {
            " ".repeat(PADDING - lhs.len())
        } else {
            "  ".to_string()
        };
        format!("{lhs}{padding}{annotation}")
    }
}
