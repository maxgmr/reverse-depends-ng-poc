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

pub use archive::*;
pub use args::*;
pub(crate) use cache::{load_cache, save_cache};
pub use output::*;
pub use parsing::*;
pub use platform_info::detect_devel_release;
pub use resolver::*;
pub use vendor::*;
