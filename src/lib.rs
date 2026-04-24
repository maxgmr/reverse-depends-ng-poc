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

pub use self::archive::*;
pub use self::args::*;
pub(crate) use self::cache::{load_cache, save_cache};
pub use self::output::*;
pub use self::parsing::*;
pub use self::platform_info::detect_devel_release;
pub use self::resolver::*;
pub use self::vendor::*;
