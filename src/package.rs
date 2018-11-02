use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::de::{self, Visitor};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct PackageData {
    path: PathBuf,
    name: String,
    version: String,
    base: Option<String>,
    description: String,
    // We require this be present even if empty as it allows protocols that rely on knowing layout.
    groups: Vec<String>,
    url: String,
    license: Option<String>,
    arch: String,
    //build_date: SystemTime,
    //install_date: SystemTime,
    packager: String,
    reason: Reason,
    size: u64,
    replaces: Vec<String>,
    depends: Vec<String>,
    optional_depends: Vec<String>,
    conflicts: Vec<String>,
    provides: Vec<String>,
    files: Vec<PackageFile>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct PackageFile;

/// Different possible validation methods
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum Validation {
    //Sha256(Array64<u8>),
// TODO Pgp,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum Reason {
    /// This package was explicitally installed
    Explicit,
    /// This package was installed because it was required for another package
    Depend,
}
