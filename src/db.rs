//! Functionality relating to alpm databases (local and sync).

use crate::error::Error;
use std::path::PathBuf;

mod local;
mod sync;

pub(crate) use self::local::LocalDatabaseInner;
pub use self::local::{InstallReason, LocalDatabase, LocalPackage, ValidationError};
pub use self::sync::{SyncDatabase, SyncPackage};
pub(crate) use self::sync::{SyncDatabaseInner, SyncDbName};

/// The name of the directory for sync databases.
pub(crate) const SYNC_DB_DIR: &str = "sync";
/// The extension of the directory for sync databases.
pub(crate) const DEFAULT_SYNC_DB_EXT: &str = "db";
/// The name of the local database.
pub(crate) const LOCAL_DB_NAME: &str = "local";

/// A trait providing all shared database functionality.
pub trait Database {
    /// The type of a package from this database.
    type Pkg;

    /// Get the name of this database
    fn name(&self) -> &str;

    /// Get the path of the root file or directory for this database.
    fn path(&self) -> PathBuf;

    /// Get the status of this database.
    fn status(&self) -> Result<DbStatus, Error>;

    /// Get the number of packages in the database
    fn count(&self) -> usize;

    /// Get a package in this database, if present.
    fn package(&self, name: impl AsRef<str>, version: impl AsRef<str>) -> Result<Self::Pkg, Error>;

    /// Get the latest version of a package in this database, if a version is present.
    fn package_latest<Str>(&self, name: Str) -> Result<Self::Pkg, Error>
    where
        Str: AsRef<str>;

    /// Run a callback on all packages in the database.
    fn packages<E, F>(&self, f: F) -> Result<(), E>
    where
        F: FnMut(Self::Pkg) -> Result<(), E>,
        E: From<Error>;
}

/// The response from checking the status of a database.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DbStatus {
    /// The database is not present.
    Missing,
    /// The database is present but invalid.
    Invalid,
    /// The database is present and valid.
    Valid,
}
/*
bitflags! {
    pub struct DbStatus: u32 {
        const VALID         = 0x0000_0001;
        const INVALID       = 0x0000_0002;
        const EXISTS        = 0x0000_0004;
        const MISSING       = 0x0000_0008;
        const LOCAL         = 0x0000_0400;
        const PACKAGE_CACHE = 0x0000_0800;
        const GROUP_CACHE   = 0x0000_1000;
    }
}
*/

bitflags::bitflags! {
    /// What this database is to be used for.
    pub struct DbUsage: u32 {
        const SYNC    = 0b0001;
        const SEARCH  = 0b0010;
        const INSTALL = 0b0100;
        const UPGRADE = 0b1000;
        const ALL     = Self::SYNC.bits |
                        Self::SEARCH.bits |
                        Self::INSTALL.bits |
                        Self::UPGRADE.bits;
    }
}

impl Default for DbUsage {
    fn default() -> Self {
        DbUsage::ALL
    }
}

/// The trust level that signatures must match.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SignatureLevel {
    /// Inherit the signature level required from the parent Alpm instance.
    Inherit,
    Optional,
    MarginalOk,
    UnknownOk,
}

impl Default for SignatureLevel {
    fn default() -> Self {
        SignatureLevel::Inherit
    }
}

/// If the name has at least 2 hyphens ('-'), split at the second from last
fn split_package_dirname(input: &str) -> Option<(&str, &str)> {
    let idx = input.rmatch_indices('-').skip(1).next()?.0;
    let start2 = idx + '-'.len_utf8();
    Some((&input[0..idx], &input[start2..]))
}

#[test]
fn test_split_package_dirname() {
    assert_eq!(
        split_package_dirname("abc-1223123-34"),
        Some(("abc", "1223123-34"))
    );
    assert_eq!(
        split_package_dirname("abc-def-1223123-34"),
        Some(("abc-def", "1223123-34"))
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test(ignore)]
    fn db_path() {
        let base_path = "/var/lib/pacman/";
        let base_path2 = "/var/lib/pacman";
        let ext = "db";

        let tests = vec![("sync1", "/var/lib/pacman/sync/sync1.db")];
        for (db_name, target) in tests {
            let db_name = SyncDbName::new(db_name).unwrap();
            let target = Path::new(target);
            assert_eq!(db_name.path(&base_path), target);
            assert_eq!(db_name.path(&base_path2), target);
        }
    }
}
