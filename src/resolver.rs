//! Module for reverse-dependency resolution. Works on lists of parsed
//! archive data, i.e., slices of [`BinaryPackage`]s and
//! [`SourcePackage`]s.

use crate::{BinaryPackage, SourcePackage};
