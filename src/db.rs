use std::cmp;
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{self, Path, PathBuf};
use std::collections::HashSet;

use error::{ErrorKind, Error};
use {Alpm, LOCAL_DB_NAME, SYNC_DB_DIR, SYNC_DB_EXT};

use atoi::atoi;
use failure::{Fail, ResultExt, err_msg};

const LOCAL_DB_VERSION_FILE: &str = "ALPM_DB_VERSION";
const LOCAL_DB_CURRENT_VERSION: u64 = 9;

/// A package database.
pub struct Db<'a> {
    base: &'a DbBase,
    handle: &'a Alpm,
    /// A cache of the database path.
    path: PathBuf,
}

impl<'a> Db<'a> {
    /// Helper to create this data structure.
    pub(crate) fn new(base: &'a DbBase, handle: &'a Alpm) -> Db<'a> {
        // todo avoid allocating this path every time we create a Db
        let path = base.name.path(&handle.database_path);
        Db { base, handle, path }
    }

    /// Get the path of this database.
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the status of the database.
    pub fn status(&self) -> Result<DbStatus, Error> {
        // alpm checks path name, but we do this during construction.

        // check if database is missing
        let metadata = match fs::metadata(self.path()) {
            Err(ref e) if e.kind() == io::ErrorKind::NotFound =>
                return Ok(DbStatus::Missing),
            Err(e) =>
                return Err(e.context(ErrorKind::CannotQueryDatabase(
                    self.base.name.to_owned()
                )).into()),
            Ok(md) => md
        };

        self.base.is_valid(metadata, self.path())
            .map(|valid| DbStatus::Exists { valid })
    }

    /// Is this database a local database
    fn is_local(&self) -> bool {
        self.base.name.is_local()
    }

    /// Is this database a sync database
    fn is_sync(&self) -> bool {
        self.base.name.is_sync()
    }


}

/// A package database.
///
/// This contains the actual database data, but is inaccessible to the user as database operations
/// in general require a handle to the main alpm instance.
pub(crate) struct DbBase {
    /// The name of the database, also used to construct the database path.
    name: DbName,
    /// The level of signature verification required to accept packages
    sig_level: SignatureLevel,
    /// Which operations this database will be used for.
    usage: DbUsage,
    /// A list of servers for this database
    servers: HashSet<String>,
}

impl DbBase {
    /// Create a new db instance
    pub(crate) fn new(
        name: DbName,
        handle: &Alpm,
        sig_level: SignatureLevel,
    ) -> Result<DbBase, ErrorKind> {
        if handle.database_exists(&name) {
            return Err(ErrorKind::DatabaseAlreadyExists(name));
        }

        Ok(DbBase::new_no_check_duplicates(name, sig_level))
    }

    /// Create a new sync db instance
    ///
    /// The name of this database must not match LOCAL_DB_NAME
    pub(crate) fn new_sync(
        name: impl AsRef<str>,
        handle: &Alpm,
        sig_level: SignatureLevel,
    ) -> Result<DbBase, ErrorKind> {
        let name = name.as_ref();
        if name == LOCAL_DB_NAME {
            return Err(ErrorKind::DatabaseAlreadyExists(DbName::LOCAL.clone()));
        }
        Self::new(
            DbName(DbNameInner::Sync(name.to_owned())),
            handle,
            sig_level,
        )
    }

    /// Create a new database without checking for duplicates.
    ///
    /// Only use this function before the alpm instance is instantiated. It is up to the
    /// caller to check there are no duplicates.
    pub(crate) fn new_no_check_duplicates(name: DbName, sig_level: SignatureLevel) -> DbBase {
        DbBase {
            name,
            sig_level,
            usage: DbUsage::ALL,
            servers: HashSet::new(),
        }
    }

    /// Get the name of the database
    #[inline]
    pub(crate) fn name(&self) -> &DbName {
        &self.name
    }

    /// Validate the database.
    ///
    /// # Params
    ///  - `md` metadata for the database root
    ///  - `path` the path of the database root
    ///
    /// Returns true if the database is valid, false otherwise
    fn is_valid(&self, md: fs::Metadata, path: &Path) -> Result<bool, Error> {

        #[inline]
        fn create_version_file(path: &Path) -> io::Result<()> {
            let mut version_file = fs::File::create(&path)?;
            // Format is number followed by single newline
            write!(version_file, "{}\n", LOCAL_DB_CURRENT_VERSION)?;
            Ok(())
        }

        if self.name.is_local() {
            if ! md.is_dir() {
                return Ok(false);
            }
            let version_path = path.join(LOCAL_DB_VERSION_FILE);
            Ok(match fs::read(&version_path) {
                Ok(version_raw) => {
                    // Check version is up to date.
                    let version: u64 = atoi(&version_raw)
                        .ok_or(format_err!(r#""{}" is not a valid version"#,
                                           String::from_utf8_lossy(&version_raw)))
                        .context(ErrorKind::DatabaseVersion(self.name.to_owned()))?;

                    version == LOCAL_DB_CURRENT_VERSION
                },
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                    // check directory is empty and create version file
                    let mut read_dir = fs::read_dir(path)
                        .context(ErrorKind::DatabaseVersion(self.name.to_owned()))?;
                    match read_dir.next() {
                        Some(_) => false,
                        None => {
                            panic!("I'm not ready to let this actually do stuff yet!");
                            create_version_file(&version_path)
                                .context(ErrorKind::DatabaseVersion(self.name.to_owned()))?;
                            true
                        }
                    }
                },
                Err(e) => return Err(e.context(
                    ErrorKind::DatabaseVersion(self.name.to_owned())
                ).into())
            })
        } else {
            // todo
            Ok(true)
        }
    }
}

/// The name (and implied type) of an alpm database.
///
/// Valid database names do not contain path separators (on any OS), or the dot char ('.').
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DbName(DbNameInner);

impl DbName {
    /// Create a new valid DbName.
    ///
    /// Returns an error if the name isn't a valid directory name.
    pub fn new(name: impl AsRef<str>) -> Result<DbName, ErrorKind> {
        let name = name.as_ref();
        let db_name = match name {
            LOCAL_DB_NAME => DbName(DbNameInner::Local),
            name if DbName::valid_syncdb_name(name) => DbName(DbNameInner::Sync(name.to_owned())),
            _ => return Err(ErrorKind::InvalidDatabaseName(name.to_owned())),
        };
        debug_assert!(db_name.is_valid());
        Ok(db_name)
    }

    /// Get the name as a string (LOCAL_DB_NAME for local, name for sync)
    #[inline]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            &DbNameInner::Local => LOCAL_DB_NAME,
            &DbNameInner::Sync(ref name) => name,
        }
    }

    /// Convert this name into a string (LOCAL_DB_NAME for local, name for sync)
    #[inline]
    pub fn into_string(self) -> String {
        match self.0 {
            DbNameInner::Local => LOCAL_DB_NAME.to_owned(),
            DbNameInner::Sync(name) => name,
        }
    }

    /// Is this the local database?
    #[inline]
    pub fn is_local(&self) -> bool {
        match &self.0 {
            &DbNameInner::Local => true,
            &DbNameInner::Sync(_) => false,
        }
    }

    /// Is is a sync database?
    #[inline]
    pub fn is_sync(&self) -> bool {
        !self.is_local()
    }

    /// Get the path for this database name
    ///
    /// Must supply the root database path from the alpm instance.
    fn path(&self, database_path: impl AsRef<Path>) -> PathBuf {
        let database_path = database_path.as_ref();
        // path is
        //  - `$db_path SEP $name` for local
        //  - `$db_path SEP "sync" SEP $name "." $ext` for sync
        match &self.0 {
            &DbNameInner::Local => database_path.join(LOCAL_DB_NAME),
            &DbNameInner::Sync(ref name) => {
                let mut path = database_path.join(SYNC_DB_DIR);
                path.push(name);
                path.set_extension(SYNC_DB_EXT);
                path
            }
        }
    }

    /// Is the string a valid sync database name?
    ///
    /// Fails if the name contains path separators (for any OS environment) or dot ('.')
    pub fn valid_syncdb_name(name: impl AsRef<str>) -> bool {
        for ch in name.as_ref().chars() {
            if path::is_separator(ch) || ch == '.' || ch == '\\' || ch == '/' {
                return false;
            }
        }
        true
    }

    /// Helper function to test whether a string is a valid directory.
    ///
    /// Available for asserts.
    fn is_valid(&self) -> bool {
        match &self.0 {
            &DbNameInner::Local => true,
            &DbNameInner::Sync(ref name) => {
                !(name == LOCAL_DB_NAME) && DbName::valid_syncdb_name(name)
            }
        }
    }

    /// The name of the local database
    pub const LOCAL: &'static DbName = &DbName(DbNameInner::Local);
}

impl fmt::Display for DbName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl AsRef<str> for DbName {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<DbName> for String {
    #[inline]
    fn from(db_name: DbName) -> String {
        db_name.into_string()
    }
}

impl cmp::PartialEq<str> for DbName {
    fn eq(&self, rhs: &str) -> bool {
        cmp::PartialEq::eq(self.as_ref(), rhs)
    }

    fn ne(&self, rhs: &str) -> bool {
        cmp::PartialEq::ne(self.as_ref(), rhs)
    }
}

impl cmp::PartialEq<DbName> for str {
    fn eq(&self, rhs: &DbName) -> bool {
        cmp::PartialEq::eq(self, rhs.as_ref())
    }

    fn ne(&self, rhs: &DbName) -> bool {
        cmp::PartialEq::ne(self, rhs.as_ref())
    }
}
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum DbNameInner {
    /// The (unique) local database.
    Local,
    /// One of the sync databases.
    Sync(String),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DbStatus {
    /// The database directory is not present.
    Missing,
    /// The database directory is present.
    Exists {
        /// Whether the database is consistent.
        valid: bool,
    },
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

bitflags! {
    /// What this database is to be used for.
    pub struct DbUsage: u32 {
        const SYNC    = 0x0000_0001;
        const SEARCH  = 0x0000_0002;
        const INSTALL = 0x0000_0004;
        const UPGRADE = 0x0000_0008;
        const ALL     = Self::SYNC.bits |
                        Self::SEARCH.bits |
                        Self::INSTALL.bits |
                        Self::UPGRADE.bits;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SignatureLevel {
    Default,
    Optional,
    MarginalOk,
    UnknownOk,
}

impl Default for SignatureLevel {
    fn default() -> Self {
        SignatureLevel::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_name() {
        assert_eq!(
            DbName::new("name_of_db").unwrap(),
            DbName(DbNameInner::Sync("name_of_db".into()))
        );
        assert_eq!(&DbName::new("local").unwrap(), DbName::LOCAL);
        assert!(DbName::new("bad/name").is_err());
        assert!(DbName::new("bad\\name").is_err());
        assert!(DbName::new("bad.name").is_err());
    }

    #[test]
    fn db_path() {
        let base_path = "/var/lib/pacman/";
        let base_path2 = "/var/lib/pacman";

        let tests = vec![
            ("local", "/var/lib/pacman/local"),
            ("sync1", "/var/lib/pacman/sync/sync1.db"),
        ];
        for (db_name, target) in tests {
            let db_name = DbName::new(db_name).unwrap();
            let target = Path::new(target);
            assert_eq!(db_name.path(&base_path), target);
            assert_eq!(db_name.path(&base_path2), target);
        }
    }
}
