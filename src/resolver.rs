//! Module for reverse-dependency resolution. Works on lists of parsed
//! archive data, i.e., slices of [`BinaryPackage`]s and
//! [`SourcePackage`]s.

use std::collections::{HashMap, HashSet};

use crate::{
    Args, BinaryPackage, SourcePackage, extract_name, parse_dep_groups, parse_dep_names,
    parse_provides,
};

/// A single reverse dependency: a package that depends on the given
/// target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevDepEntry<'a> {
    /// Name of the package.
    pub package: &'a str,
    /// Architectures on which this dependency exists.
    pub architectures: Vec<&'a str>,
    /// Archive component.
    pub component: &'a str,
    /// The raw dependency expression.
    pub dependency: &'a str,
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

/// A pre-built reverse dependency index for fast lookups.
///
/// Built once from the full package lists via [`ReverseIndex::build`];
/// each subsequent call to [`find_rev_deps`] looks up targets in O(1)
/// rather than scanning every package.
#[derive(Debug, Clone)]
pub struct ReverseIndex<'a> {
    /// Map of each dependency name to every binary-package relationship
    /// that references it.
    pub binary_map: HashMap<&'a str, Vec<BinRevRef<'a>>>,
    /// Map of each dependency name to every source-package relationship
    /// that references it.
    pub source_map: HashMap<&'a str, Vec<SrcRevRef<'a>>>,
}
impl<'a> ReverseIndex<'a> {
    /// Build a reverse index from the full lists of binary and source
    /// packages.
    #[must_use]
    pub fn build(binaries: &'a [BinaryPackage], sources: &'a [SourcePackage]) -> Self {
        let mut binary_map: HashMap<&str, Vec<BinRevRef<'_>>> =
            HashMap::with_capacity(binaries.len());
        let mut source_map: HashMap<&str, Vec<SrcRevRef<'_>>> =
            HashMap::with_capacity(sources.len());

        // Index all four binary relationship types
        for bin in binaries {
            let fields: &[(&String, &'static str)] = &[
                (&bin.depends, "Reverse-Depends"),
                (&bin.pre_depends, "Reverse-Pre-Depends"),
                (&bin.recommends, "Reverse-Recommends"),
                (&bin.suggests, "Reverse-Suggests"),
            ];
            for &(raw_field, group) in fields {
                if raw_field.is_empty() {
                    continue;
                }

                for (raw_or_group, dep_names) in parse_dep_groups(raw_field) {
                    for dep_name in dep_names {
                        binary_map.entry(dep_name).or_default().push(BinRevRef {
                            package: bin,
                            group,
                            dep_expr: raw_or_group,
                        });
                    }
                }
            }
        }

        // Index source build-dep fields and testsuite triggers
        for src in sources {
            let fields: &[(&String, &'static str)] = &[
                (&src.build_depends, "Reverse-Build-Depends"),
                (&src.build_depends_indep, "Reverse-Build-Depends-Indep"),
                (&src.build_depends_arch, "Reverse-Build-Depends-Arch"),
            ];
            for &(raw_field, group) in fields {
                if raw_field.is_empty() {
                    continue;
                }

                for (raw_or_group, dep_names) in parse_dep_groups(raw_field) {
                    for dep_name in dep_names {
                        source_map.entry(dep_name).or_default().push(SrcRevRef {
                            package: src,
                            group,
                            dep_expr: raw_or_group,
                        });
                    }
                }
            }

            // Testsuite-Triggers: index each named trigger directly,
            // and expand `@builddeps@` so that any build dep also acts
            // as a trigger.
            if !src.testsuite_triggers.is_empty() {
                let triggers: Vec<&str> = src
                    .testsuite_triggers
                    .split(',')
                    .filter_map(extract_name)
                    .collect();

                let has_builddeps = triggers.contains(&"@builddeps@");

                let mut trigger_dep_names: HashSet<&str> = triggers
                    .into_iter()
                    .filter(|&t| t != "@builddeps@")
                    .collect();

                if has_builddeps {
                    for field in [
                        &src.build_depends,
                        &src.build_depends_indep,
                        &src.build_depends_arch,
                    ] {
                        for or_group in parse_dep_names(field) {
                            trigger_dep_names.extend(or_group);
                        }
                    }
                }

                for dep_name in trigger_dep_names {
                    source_map.entry(dep_name).or_default().push(SrcRevRef {
                        package: src,
                        group: "Reverse-Testsuite-Triggers",
                        dep_expr: "",
                    });
                }
            }
        }

        Self {
            binary_map,
            source_map,
        }
    }
}

/// A single binary package -> dependency name relationship stored in
/// the index.
#[derive(Debug, Clone)]
pub struct BinRevRef<'a> {
    /// The binary package that declares this relationship.
    pub package: &'a BinaryPackage,
    /// Output group name, e.g. `"Reverse-Depends"`.
    pub group: &'static str,
    /// Full OR-expression, e.g. `"libfoo | libbar"`.
    pub dep_expr: &'a str,
}

/// A single source package -> dependency name relationship stored in
/// the index.
#[derive(Debug, Clone)]
pub struct SrcRevRef<'a> {
    /// The source package that declares this relationship.
    pub package: &'a SourcePackage,
    /// Output group name, e.g. `"Reverse-Build-Depends"`.
    pub group: &'static str,
    /// Full OR-expression; always empty for
    /// `Reverse-Testsuite-Triggers`
    pub dep_expr: &'a str,
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
            parse_provides(&p.provides)
                .into_iter()
                .map(ToString::to_string)
        })
        .collect()
}

/// Find all reverse dependencies of `target_names` in the archive.
///
/// Returns the sorted list of [`RevDepEntry`] values, grouped by their
/// relationship group name; e.g., `"Reverse-Depends"`.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn find_rev_deps<'a>(
    index: &'a ReverseIndex<'a>,
    target_names: &HashSet<&str>,
    args: &Args,
) -> HashMap<&'static str, Vec<RevDepEntry<'a>>> {
    // Accumulator:
    // output-field-name -> (package-name, dep-expr) -> entry.
    //
    // The compound key deduplicates across two dimensions:
    //   1. Same package in multiple Packages.gz (diff arches): merge
    //      into one entry, appending to `architectures`
    //   2. Same package with distinct dep expresssions: separate
    //      entries
    let mut acc: HashMap<&'static str, HashMap<(&str, &str), RevDepEntry<'_>>> = HashMap::new();

    if args.want_build_depends() {
        // Get build deps
        for &target in target_names {
            let Some(refs) = index.source_map.get(target) else {
                continue;
            };
            for r in refs {
                // Testsuite-Triggers entries always use an empty
                // dep_expr as the key, so a source package gets at most
                // one entry in "Reverse-Testsuite-Triggers" regardless
                // of how many targets matched it.
                acc.entry(r.group)
                    .or_default()
                    .entry((r.package.name.as_str(), r.dep_expr))
                    .or_insert_with(|| RevDepEntry {
                        package: &r.package.name,
                        architectures: vec!["source"],
                        component: &r.package.component,
                        dependency: r.dep_expr,
                    });
            }
        }
    } else {
        // Get runtime deps
        for &target in target_names {
            let Some(refs) = index.binary_map.get(target) else {
                continue;
            };
            for r in refs {
                let enabled = match r.group {
                    "Reverse-Recommends" => args.recommends,
                    "Reverse-Suggests" => args.suggests,
                    _ => true,
                };
                if !enabled {
                    continue;
                }
                let entry = acc
                    .entry(r.group)
                    .or_default()
                    .entry((r.package.name.as_str(), r.dep_expr))
                    .or_insert_with(|| RevDepEntry {
                        package: &r.package.name,
                        architectures: Vec::new(),
                        component: &r.package.component,
                        dependency: r.dep_expr,
                    });
                if !entry.architectures.contains(&r.package.arch.as_str()) {
                    entry.architectures.push(&r.package.arch);
                }
            }
        }
    }

    // Convert inner HashMaps to sorted Vecs for deterministic output
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

/// Build the full reverse dependency tree up to the given maximum
/// depth.
///
/// Returns a map from each package name to its own reverse dependency
/// results. The entry keyed by `queried_package` is the root; the
/// display layer can then walk the tree recursively by looking up each
/// package's own entry.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn find_rev_deps_recursive<'a>(
    index: &'a ReverseIndex<'a>,
    queried_package: &'a str,
    initial_targets: &HashSet<&str>,
    args: &Args,
) -> HashMap<&'a str, HashMap<&'static str, Vec<RevDepEntry<'a>>>> {
    let mut all_results = HashMap::new();

    // Pre-populate visited with initial targets
    let mut visited: HashSet<&str> = initial_targets.clone();

    // Depth 0: query all initial targets together. Identical to non-
    // recursive mode.
    let root_rev_deps = find_rev_deps(index, initial_targets, args);

    // Seed first frontier with all packages found at depth 0, excluding
    // anything already visited.
    let mut frontier: HashSet<&str> = root_rev_deps
        .values()
        .flat_map(|entries| entries.iter())
        .map(|entry| entry.package)
        .filter(|name| visited.insert(name))
        .collect();

    all_results.insert(queried_package, root_rev_deps);

    // BFS: each iteration processes one depth level.
    for _ in 1..=args.recursive_depth {
        if frontier.is_empty() {
            break;
        }

        let mut next_frontier = HashSet::new();

        for &package in &frontier {
            let single_target = HashSet::from([package]);
            let rev_deps = find_rev_deps(index, &single_target, args);

            // Gather newly discovered packages for next depth level
            for entries in rev_deps.values() {
                for entry in entries {
                    if visited.insert(entry.package) {
                        next_frontier.insert(entry.package);
                    }
                }
            }

            if !rev_deps.is_empty() {
                all_results.insert(package, rev_deps);
            }
        }

        frontier = next_frontier;
    }

    all_results
}

// AI-generated unit tests
#[cfg(test)]
#[path = "unit_tests/resolver_tests.rs"]
mod tests;
