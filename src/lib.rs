//! # reverse-depends-ng-poc
//!
//! Proof of concept for a modernized reverse-depends.
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::todo)]

mod archive;
mod args;
mod cache;
mod output;
mod parsing;
mod platform_info;
mod resolver;
mod vendor;

pub use self::archive::{fetch_binaries, fetch_sources};
pub use self::args::{ArchSearchCombo, Args};
pub(crate) use self::cache::{load_cache, save_cache};
pub use self::output::{
    list_output, list_output_recursive, verbose_output, verbose_output_recursive,
};
pub use self::parsing::{BinaryPackage, SourcePackage};
pub(crate) use self::parsing::{
    extract_name, parse_binary_packages, parse_dep_groups, parse_dep_names, parse_provides,
    parse_source_packages,
};
pub use self::platform_info::detect_devel_release;
pub use self::resolver::{
    RevDepEntry, ReverseIndex, binaries_provides, find_rev_deps, find_rev_deps_recursive,
    source_binaries,
};
pub use self::vendor::Vendor;
