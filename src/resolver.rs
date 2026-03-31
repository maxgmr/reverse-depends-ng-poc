//! Module for reverse-dependency resolution. Works on lists of parsed
//! archive data, i.e., slices of [`BinaryPackage`]s and
//! [`SourcePackage`]s.

use std::collections::{HashMap, HashSet};

use crate::{Args, BinaryPackage, SourcePackage, extract_name, parse_dep_names};

/// A single reverse dependency: a package that depends on the given
/// target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevDepEntry<'a> {
    /// Name of the package.
    pub package: &'a str,
    /// Architectures on which this dependency exists.
    pub architectures: Vec<&'static str>,
    /// Archive component.
    pub component: &'static str,
    /// The raw dependency expression.
    pub dependency: String,
}
impl PartialOrd for RevDepEntry<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for RevDepEntry<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.package.cmp(other.package)
    }
}

/// Return all binary package names produced by the source package with
/// the given name.
#[must_use]
pub fn source_binaries(sources: &[SourcePackage], source_name: &str) -> HashSet<String> {
    sources
        .iter()
        .filter(|s| s.name == source_name)
        .flat_map(|s| {
            s.binaries
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .collect()
}

/// Return all virtual package names provided by packages in
/// `target_names`.
#[allow(clippy::implicit_hasher)]
#[must_use]
pub fn binaries_provides(
    binaries: &[BinaryPackage],
    target_names: &HashSet<String>,
) -> HashSet<String> {
    binaries
        .iter()
        .filter(|p| target_names.contains(&p.name))
        .flat_map(|p| {
            p.provides
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .collect()
}

/// Find all reverse dependencies of `target_names` in the archive.
///
/// Returns the sorted list of [`RevDepEntry`] values, sorted by their
/// relationship group name; e.g., `"Reverse-Depends"`.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn find_rev_deps<'a>(
    binaries: &'a [BinaryPackage],
    sources: &'a [SourcePackage],
    target_names: &HashSet<String>,
    args: &Args,
) -> HashMap<&'static str, Vec<RevDepEntry<'a>>> {
    // Accumulator: output-field-name -> (package-name -> entry).
    //
    // This is keyed by (package_name, dep_expression).
    // package_name is there so when we see the same package in multiple
    // Packages.gzs, we can append to its arches list rather than
    // creating duplicates. However, dep_expression is there so *all*
    // relationships are kept if a package depends on multiple packages
    // in target_names.
    let mut acc: HashMap<&'static str, HashMap<(&str, String), RevDepEntry<'_>>> = HashMap::new();

    if !args.want_build_depends() {
        for bin in binaries {
            // Create table of:
            //   - The package's raw dependency field
            //   - The relationship group under which it belongs
            //   - Whether or not this dependency category is enabled
            let fields: &[(&String, &'static str, bool)] = &[
                (&bin.depends, "Reverse-Depends", true),
                (&bin.pre_depends, "Reverse-Pre-Depends", true),
                (&bin.recommends, "Reverse-Recommends", args.recommends),
                (&bin.suggests, "Reverse-Suggests", args.suggests),
            ];

            for &(raw_field, group, enabled) in fields {
                if !enabled || raw_field.is_empty() {
                    continue;
                }

                for or_group in parse_dep_names(raw_field) {
                    // Skip anything already in the target set
                    if !or_group.iter().any(|&n| target_names.contains(n)) {
                        continue;
                    }

                    // The full OR expression is stored for display purposes.
                    let dep_expr = or_group.join(" | ");

                    let group_entry = acc.entry(group).or_default();
                    let entry = group_entry
                        .entry((&bin.name, dep_expr.clone()))
                        .or_insert_with(|| RevDepEntry {
                            package: &bin.name,
                            architectures: Vec::new(),
                            component: bin.component,
                            dependency: dep_expr,
                        });
                    if !entry.architectures.contains(&bin.arch) {
                        entry.architectures.push(bin.arch);
                    }
                }
            }
        }
    }

    // Source package build-dependency fields
    if args.want_build_depends() {
        for src in sources {
            // Create table of:
            //   - The package's raw dependency field
            //   - The relationship group under which it belongs
            let fields: &[(&String, &'static str)] = &[
                (&src.build_depends, "Reverse-Build-Depends"),
                (&src.build_depends_indep, "Reverse-Build-Depends-Indep"),
                (&src.build_depends_arch, "Reverse-Build-Depends-Arch"),
            ];

            for &(raw_field, group) in fields {
                if raw_field.is_empty() {
                    continue;
                }

                for or_group in parse_dep_names(raw_field) {
                    // Skip anything already in the target set
                    if !or_group.iter().any(|&n| target_names.contains(n)) {
                        continue;
                    }

                    let dep_expr = or_group.join(" | ");
                    acc.entry(group)
                        .or_default()
                        .entry((&src.name, dep_expr.clone()))
                        .or_insert_with(|| RevDepEntry {
                            package: &src.name,
                            architectures: vec!["source"],
                            component: src.component,
                            dependency: dep_expr,
                        });
                }
            }

            // Handle testsuite triggers
            if !src.testsuite_triggers.is_empty() {
                let triggers: Vec<&str> = src
                    .testsuite_triggers
                    .split(',')
                    .filter_map(extract_name)
                    .collect();

                let direct_match = triggers.iter().any(|&t| target_names.contains(t));

                // When a source package has
                // "Testsuite-Triggers: @builddeps@", it means "re-run
                // my tests whenever any of my build deps change". We
                // must expand `@builddeps@` into those build
                // dependencies.
                let builddeps_match = triggers.contains(&"@builddeps@")
                    && [
                        &src.build_depends,
                        &src.build_depends_indep,
                        &src.build_depends_arch,
                    ]
                    .iter()
                    .any(|f| field_matches_target(f, target_names));

                if direct_match || builddeps_match {
                    let group_entry = acc.entry("Reverse-Testsuite-Triggers").or_default();
                    group_entry
                        // OK to use String::new() as dep component
                        // because search source package gets only one
                        // Testsuite-Triggers entry regardless of how
                        // many triggers matched.
                        .entry((&src.name, String::new()))
                        .or_insert_with(|| RevDepEntry {
                            package: &src.name,
                            architectures: vec!["source"],
                            component: src.component,
                            dependency: String::new(),
                        });
                }
            }
        }
    }

    // Convert the inner HashMaps to sorted Vecs to ensure deterministic
    // output regardless of download order.
    acc.into_iter()
        .map(|(field, pkg_map)| {
            let mut entries: Vec<RevDepEntry<'_>> = pkg_map.into_values().collect();
            entries.sort_unstable();
            for e in &mut entries {
                e.architectures.sort_unstable();
            }
            (field, entries)
        })
        .collect()
}

/// Return true if and only if `target_names` contains an item in the
/// given field.
fn field_matches_target(field: &str, target_names: &HashSet<String>) -> bool {
    !field.is_empty()
        && parse_dep_names(field)
            .iter()
            .any(|or_group| or_group.iter().any(|&n| target_names.contains(n)))
}
