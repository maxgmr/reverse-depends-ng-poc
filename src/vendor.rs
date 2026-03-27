//! This module contains the [`Vendor`] enum, which defines the info
//! for the different supported archives.

/// The different vendors supported by this program.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
#[allow(missing_docs)]
pub enum Vendor {
    Debian,
    #[default]
    Ubuntu,
}
impl Vendor {
    /// Get this vendor's primary archive URL.
    #[must_use]
    pub fn archive(&self) -> &'static str {
        match self {
            Self::Debian => "http://ftp.debian.org/debian",
            Self::Ubuntu => "http://archive.ubuntu.com/ubuntu",
        }
    }

    /// Get this vendor's ports archive URL.
    #[must_use]
    pub fn ports(&self) -> &'static str {
        match self {
            Self::Debian => "http://ftp.ports.debian.org/debian-ports",
            Self::Ubuntu => "http://ports.ubuntu.com/ubuntu-ports",
        }
    }

    // TODO get this dynamically from the archive instead
    /// Get the architectures supported by this vendor's primary archive.
    #[must_use]
    pub fn primary_arches(&self) -> &'static [&'static str] {
        match self {
            Self::Debian => &["all", "amd64", "i386"],
            Self::Ubuntu => &["amd64", "amd64v3", "i386"],
        }
    }

    // TODO get this dynamically from the archive instead
    /// Get the architectures supported by the given release's ports archive.
    #[must_use]
    pub fn ports_arches(&self, release: &str) -> &'static [&'static str] {
        match (self, release) {
            (Self::Ubuntu, "trusty") => &["arm64", "armhf", "powerpc", "ppc64el"],
            (Self::Ubuntu, "xenial") => &["arm64", "armhf", "powerpc", "ppc64el", "s390x"],
            (Self::Ubuntu, "bionic") => &["arm64", "armhf", "ppc64el", "s390x"],
            (Self::Debian | Self::Ubuntu, _) => &["arm64", "armhf", "ppc64el", "riscv64", "s390x"],
        }
    }

    /// Get the components supported by this vendor.
    #[must_use]
    pub fn components(&self) -> &'static [&'static str] {
        match self {
            Self::Debian => &["main", "contrib", "non-free", "non-free-firmware"],
            Self::Ubuntu => &["main", "restricted", "universe", "multiverse"],
        }
    }
}
