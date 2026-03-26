//! This module contains CLI argument handling.

use clap::Parser;

const ARCH_DEFAULT: &str = "any";

#[allow(clippy::doc_markdown)]
/// List reverse-dependencies of an Ubuntu/Debian package.
///
/// If PACKAGE is prefixed with `src:`, the reverse-dependencies of all
/// binary packages produced by that source package will be listed.
///
/// Unlike the original `reverse-depends`, this tool queries the Ubuntu
/// archive directly rather than relying on the UbuntuWire web service.
/// This correctly handles virtual packages, `Provides:` declarations,
/// and the Rust crate ecosystem's use of `Provides` in build
/// dependencies.
#[derive(Parser, Debug)]
#[command(name = "reverse-depends", author, version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
pub struct Args {
    /// Package to query (prefix with `src:` for source packages)
    pub package: String,
    /// Query dependencies in RELEASE (default: current devel release)
    #[arg(short, long)]
    pub release: Option<String>,
    /// Distro to check (default: ubuntu; supported: ubuntu, debian)
    #[arg(short = 'V', long, default_value_t = String::from("ubuntu"))]
    pub vendor: String,
    /// Ignore Recommends relationships. By default, Recommends are
    /// included along with Depends.
    #[arg(short = 'R', long = "without-recommends", action = clap::ArgAction::SetFalse)]
    pub recommends: bool,
    /// Also consider Suggests relationships.
    #[arg(short = 's', long = "with-suggests")]
    pub suggests: bool,
    /// Also consider Provides relationships.
    #[arg(short = 'p', long = "with-provides")]
    pub provides: bool,
    /// Query build dependencies instead of binary dependencies.
    /// Equivalent to `--arch=source`.
    #[arg(short, long = "build-depends")]
    pub build_depends: bool,
    /// Query dependencies in ARCH, or `source` for build dependencies.
    /// Default `any` queries all architectures.
    #[arg(short, long, default_value = ARCH_DEFAULT)]
    pub arch: Vec<String>,
    /// Skip ports architectures.
    #[arg(long = "no-parts", action = clap::ArgAction::SetFalse)]
    pub ports: bool,
    /// Only consider reverse dependencies in COMPONENT (repeatable).
    #[arg(short, long = "component")]
    pub components: Vec<String>,
    /// Display a simple, machine-readable list.
    #[arg(short, long)]
    pub list: bool,
}
