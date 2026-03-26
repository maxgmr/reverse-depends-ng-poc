//! This module contains CLI argument handling.

use clap::Parser;

const ARCHIVE_BASE_DEFAULT: &str = "http://archive.ubuntu.com/ubuntu";
const ARCH_DEFAULT: &str = "any";

#[allow(clippy::doc_markdown)]
/// List reverse-dependencies of an Ubuntu package.
///
/// If PACKAGE is prefixed with `src:`, the reverse-dependencies of all
/// binary packages produced by that source package will be listed.
///
/// Unlike the original `reverse-depends`, this tool queries the Ubuntu
/// archive directly rather than relying on the UbuntuWire web service.
/// This correctly handles virtual packages, `Provides:` declarations,
/// and the Rust crate ecosystem's use of `Provides` in build
/// dependencies.
///
/// Archives queried by default:
/// - `<http://archive.ubuntu.com/ubuntu>`      (amd64, i386)
/// - `<http://ports.ubuntu.com/ubuntu-ports>`  (arm64, armhf, ppc64el, riscv64, s390x)
#[derive(Parser, Debug)]
#[command(name = "reverse-depends", author, version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
pub struct Args {
    /// Package to query (prefix with `src:` for source packages)
    pub package: String,
    /// Query dependencies in RELEASE (default: current devel release)
    #[arg(short, long)]
    pub release: Option<String>,
    /// Archive base directory or URL (default: Ubuntu archive URL)
    #[arg(short = 'B', long = "archive-base", default_value_t = String::from(ARCHIVE_BASE_DEFAULT))]
    pub archive_base: String,
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
