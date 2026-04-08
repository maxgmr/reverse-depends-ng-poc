//! This module contains all functionality responsible for formatting
//! the output.

use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    hash::BuildHasher,
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
    "Reverse-Testsuite-Triggers",
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
pub fn verbose_output<S: BuildHasher>(
    queried_package: &str,
    result: &HashMap<&'static str, Vec<RevDepEntry<'_>>, S>,
) -> String {
    let mut output = String::new();
    let all_arches: HashSet<&str> = result
        .values()
        .flat_map(|entries| entries.iter())
        .flat_map(|e| e.architectures.iter().copied())
        .filter(|&a| a != "source")
        .collect();

    write_sections(result, &all_arches, queried_package, &mut output, |_, _| {});
    write_arch_footer(&all_arches, &mut output);
    output
}

/// Get the formatted output string for recursive mode.
///
/// Each root-level entry is immediately followed by its transitive
/// reverse dependency subtree, indented accordingly.
#[must_use]
pub fn verbose_output_recursive<'a, S: BuildHasher>(
    queried_package: &str,
    all_results: &HashMap<&'a str, HashMap<&'static str, Vec<RevDepEntry<'a>>, S>, S>,
) -> String {
    let mut output = String::new();

    let Some(root_results) = all_results.get(queried_package) else {
        return output;
    };

    // Collect arches across all depth labels so arch labels are
    // consistent
    let all_arches: HashSet<&str> = all_results
        .values()
        .flat_map(|results| results.values())
        .flat_map(|entries| entries.iter())
        .flat_map(|e| e.architectures.iter().copied())
        .filter(|&a| a != "source")
        .collect();

    // Prevent a package's subtree from being rendered more than once if
    // it appears under multiple parents
    let mut visited = HashSet::from([queried_package]);

    write_sections(
        root_results,
        &all_arches,
        queried_package,
        &mut output,
        |entry, output| {
            if visited.insert(entry.package) {
                render_subtree(
                    entry.package,
                    all_results,
                    &all_arches,
                    &mut visited,
                    1,
                    output,
                );
            }
        },
    );
    write_arch_footer(&all_arches, &mut output);
    output
}

/// Print a simple, deduplicated, newline-separated list of package
/// names. Ideal for scripting.
///
/// All relationship groups are merged; each package name appears at
/// most once.
#[must_use]
pub fn list_output<S: BuildHasher>(
    result: &HashMap<&'static str, Vec<RevDepEntry<'_>>, S>,
) -> String {
    collect_and_sort_names(
        result
            .values()
            .flat_map(|entries| entries.iter().map(|e| e.package)),
    )
}

/// Print a simple, deduplicated, newline-separated list of transitively
/// found package names. Ideal for scripting.
///
/// All relationship groups are merged and all transitive relationships
/// are flattened.
#[must_use]
pub fn list_output_recursive<'a, S: BuildHasher>(
    all_results: &HashMap<&'a str, HashMap<&'static str, Vec<RevDepEntry<'a>>, S>, S>,
) -> String {
    collect_and_sort_names(
        all_results
            .values()
            .flat_map(|inner| inner.values())
            .flat_map(|entries| entries.iter().map(|e| e.package)),
    )
}

/// Helper for [`list_output`] and [`list_output_recursive`].
fn collect_and_sort_names<'a>(iter: impl Iterator<Item = &'a str>) -> String {
    let mut names: Vec<&'a str> = iter.collect::<HashSet<_>>().into_iter().collect();
    names.sort_unstable();
    names.join("\n")
}

/// Helper function which handles the section-rendering loop of
/// [`verbose_output`] and [`verbose_output_recursive`]. The only
/// difference is the recursive version needs to call `render_subtree`
/// after each entry, which can be passed as a closure to this function.
fn write_sections<'a, F, S: BuildHasher>(
    result: &HashMap<&'static str, Vec<RevDepEntry<'a>>, S>,
    all_arches: &HashSet<&str>,
    queried_package: &str,
    output: &mut String,
    mut after_entry: F,
) where
    F: FnMut(&RevDepEntry<'a>, &mut String),
{
    for field in ordered_fields(result) {
        // Add header
        output.push_str(field);
        output.push('\n');
        output.push_str(&"=".repeat(field.len()));
        output.push('\n');

        // Add entries, potentially adding more things for each entry
        for entry in &result[field] {
            output.push_str(&format_entry(entry, all_arches, queried_package, 0));
            output.push('\n');
            after_entry(entry, output);
        }

        // Add blank line after section
        output.push('\n');
    }
}

/// Recursively render the subtree of reverse dependencies for
/// `package`, indented according to `depth`. Children are rendered
/// across all relationship trees without sub-headers to keep the tree
/// readable.
fn render_subtree<'a, S: BuildHasher>(
    package: &str,
    all_results: &HashMap<&'a str, HashMap<&'static str, Vec<RevDepEntry<'a>>, S>, S>,
    all_arches: &HashSet<&str>,
    visited: &mut HashSet<&'a str>,
    depth: usize,
    output: &mut String,
) {
    let Some(results) = all_results.get(package) else {
        return;
    };

    for field in ordered_fields(results) {
        for entry in &results[field] {
            // Package `package` (not orig root) as `queried_package` so
            // the "(for ...)" annotation is correctly suppressed when
            // the dependency matches the immediate parent.
            output.push_str(&format_entry(entry, all_arches, package, depth));
            output.push('\n');

            if visited.insert(entry.package) {
                render_subtree(
                    entry.package,
                    all_results,
                    all_arches,
                    visited,
                    depth + 1,
                    output,
                );
            }
        }
    }
}

/// Return the result's field names in the original display order.
///
/// Any fields missing from [`FIELD_ORDER`] are appended alphabetically.
fn ordered_fields<S: BuildHasher>(
    result: &HashMap<&'static str, Vec<RevDepEntry<'_>>, S>,
) -> Vec<&'static str> {
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

/// Write the footer, to `output`, which denotes the architectures the
/// unlabelled packages are reverse dependencies on.
fn write_arch_footer(all_arches: &HashSet<&str>, output: &mut String) {
    if !all_arches.is_empty() {
        let mut sorted: Vec<&str> = all_arches.iter().copied().collect();
        sorted.sort_unstable();
        let _ = write!(
            output,
            "Packages without architectures listed are reverse-dependencies in: {}",
            sorted.join(", ")
        );
    }
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
    all_arches: &HashSet<&str>,
    queried_package: &str,
    depth: usize,
) -> String {
    let indent = "  ".repeat(depth);

    let arch_label = {
        let entry_arch_set: HashSet<&str> = entry.architectures.iter().copied().collect();
        let all_arch_set: HashSet<&str> = all_arches.iter().copied().collect();
        let only_source_set: HashSet<&str> = HashSet::from(["source"]);
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

    let lhs = format!("{}* {}{}", indent, entry.package, arch_label);

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

#[cfg(test)]
#[path = "unit_tests/output_tests.rs"]
mod tests;
