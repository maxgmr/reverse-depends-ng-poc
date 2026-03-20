//! This module contains everything related to the command-line interface.

use clap::Parser;

/// This avoids infinite loops.
const DEFAULT_MAX_DEPTH: u32 = 10;

/// List the reverse-dependencies (or build-dependencies) of a package.
///
/// If the package name is prefixed with src: then the
/// reverse-dependencies of all the binary packages that the specified
/// source package builds will be listed.
#[derive(Debug, Parser)]
#[clap(
    author = "Max Gilmour <max.gilmour@canonical.com>",
    about = "List the reverse-dependencies (or build-dependencies) of a package."
)]
#[command(version, about)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    /// Package to query (prefix with src: for source packages)
    pub package: String,

    /// Query dependencies in RELEASE [default: current devel release]
    #[arg(short, long)]
    pub release: Option<String>,

    /// Only consider Depends relationships, not Recommends
    #[arg(short = 'R', long = "without-recommends", action = clap::ArgAction::SetFalse)]
    pub recommends: bool,

    /// Also consider Suggests relationships
    #[arg(short = 's', long = "with-suggests")]
    pub with_suggests: bool,

    /// Query build dependencies (synonym for --arch=source)
    #[arg(short = 'b', long = "build-depends")]
    pub build_depends: bool,

    /// Query dependencies in ARCH. Default: any
    #[arg(short, long, default_value_t = String::from("any"))]
    pub arch: String,

    /// Only consider reverse-dependencies in COMPONENT (repeatable)
    #[arg(short, long = "component")]
    pub components: Vec<String>,

    /// Display a simple, machine-readable list
    #[arg(short, long)]
    pub list: bool,

    #[allow(clippy::doc_markdown)]
    /// Reverse Dependencies webservice URL. Default: UbuntuWire
    #[arg(short = 'u', long = "service-url", value_name = "URL")]
    pub server: Option<String>,

    /// Find reverse dependencies recursively
    #[arg(short = 'x', long)]
    pub recursive: bool,

    /// Maximum recursion depth (requires --recursive to be enabled)
    #[arg(short = 'd', long, default_value_t = DEFAULT_MAX_DEPTH, requires = "recursive")]
    pub recursive_depth: u32,
}
