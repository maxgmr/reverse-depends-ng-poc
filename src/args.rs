//! This module contains CLI argument handling.

use crate::Vendor;

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
#[command(name = "reverse-depends", author, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
pub struct Args {
    /// Package to query (prefix with `src:` for source packages)
    pub package: String,
    /// Query dependencies in RELEASE [default: current devel release]
    #[arg(short, long)]
    pub release: Option<String>,
    /// Distro to check
    #[arg(short = 'V', long, value_enum, default_value_t = Vendor::Ubuntu)]
    pub vendor: Vendor,
    /// Ignore Recommends relationships
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
    #[arg(short, long, default_value = ARCH_DEFAULT)]
    pub arch: Vec<String>,
    /// Skip ports architectures.
    #[arg(long = "no-parts", action = clap::ArgAction::SetFalse)]
    pub ports: bool,
    /// Only consider reverse dependencies in COMPONENT (repeatable).
    #[arg(short, long = "component")]
    pub components: Vec<String>,
    // TODO add proposed argument
    // TODO add "only consider reverse dependencies in POCKET (repeatable)"
    /// Display a simple, machine-readable list.
    #[arg(short, long)]
    pub list: bool,
}

/// Get the list of components in the selected [`Vendor`] which
/// were selected by [`Args::components`].
///
/// If [`Args::components`] is empty, then all components in the
/// selected [`Vendor`] are selected.
///
/// # Errors
///
/// This function returns an [`anyhow::Error`] if the returned list is
/// empty.
pub fn get_selected_components(args: &Args) -> anyhow::Result<Vec<&'static str>> {
    #[allow(clippy::used_underscore_items)]
    _get_selected_components(args.vendor.components(), &args.components)
}

/// Helper to make testing easier
fn _get_selected_components(
    vendor_components: &'static [&'static str],
    arg_components: &[String],
) -> anyhow::Result<Vec<&'static str>> {
    if arg_components.is_empty() {
        return Ok(vendor_components.to_vec());
    }

    let result: Vec<&'static str> = vendor_components
        .iter()
        .copied()
        .filter(|&vendor_component| {
            arg_components
                .iter()
                .any(|arg_component| arg_component.as_str() == vendor_component)
        })
        .collect();

    if result.is_empty() {
        anyhow::bail!("No components named {arg_components:?} exist");
    }

    Ok(result)
}

#[cfg(test)]
#[allow(clippy::used_underscore_items)]
mod tests {
    use super::*;

    #[test]
    fn all_ubuntu_components() {
        assert_eq!(
            _get_selected_components(Vendor::Ubuntu.components(), &[]).unwrap(),
            Vendor::Ubuntu.components()
        );
    }

    #[test]
    fn all_debian_components() {
        assert_eq!(
            _get_selected_components(Vendor::Debian.components(), &[]).unwrap(),
            Vendor::Debian.components()
        );
    }

    #[test]
    fn invalid_components() {
        _get_selected_components(
            Vendor::Ubuntu.components(),
            &[String::from("nonexistent component")],
        )
        .unwrap_err();
    }

    #[test]
    fn some_ubuntu_components() {
        assert_eq!(
            _get_selected_components(
                Vendor::Ubuntu.components(),
                &[String::from("main"), String::from("restricted")]
            )
            .unwrap(),
            vec!["main", "restricted"]
        );
    }

    #[test]
    fn one_valid_component() {
        assert_eq!(
            _get_selected_components(
                Vendor::Debian.components(),
                &[
                    String::from("nonexistent component"),
                    String::from("non-free-firmware")
                ]
            )
            .unwrap(),
            vec!["non-free-firmware"]
        );
    }
}
