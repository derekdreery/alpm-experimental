//! Functionality relating to alpm databases (local and sync).
use std::borrow::Cow;
use std::cell::{self, RefCell};
use std::collections::{HashSet, HashMap};
use std::convert::TryInto;
use std::cmp;
use std::fmt::{self, Display};
use std::fs;
use std::io::{self, Read, Write};
use std::ops::Deref;
use std::path::{self, Path, PathBuf};
use std::rc::Rc;

use error::{ErrorKind, Error};
use {Alpm, LOCAL_DB_NAME, SYNC_DB_DIR};

use atoi::atoi;
use failure::{Fail, ResultExt, err_msg};
use fs2::FileExt;
use reqwest::{self, Url};
use spin::Once;

const LOCAL_DB_VERSION_FILE: &str = "ALPM_DB_VERSION";
const LOCAL_DB_CURRENT_VERSION: u64 = 9;

pub trait Database: Debug {
    /// Get the name of this database
    pub fn name(&self) -> &str;

    /// Get the path of the root file or directory for this database.
    pub fn path(&self) -> &Path;

    /// Get the status of this database.
    pub fn status(&self) -> DbStatus;

    /// Synchronize the database with any external sources.
    pub fn synchronize(&self) {
        // do nothing by default
    }

    /// Get the packages in this database
    pub fn packages(&self) -> &HashMap<String, Package> {
        unimplemented!();
    }
}


/// A package database.
pub struct SyncDatabase {
    /// Handle to the alpm instance
    handle: Rc<RefCell<Handle>>,
    /// The name of the database, also used to construct the database path.
    name: DbName,
    /// The level of signature verification required to accept packages
    sig_level: SignatureLevel,
    /// Which operations this database will be used for.
    usage: DbUsage,
    /// A list of servers for this database
    servers: RefCell<HashSet<Url>>,
    /// The database path.
    path: PathBuf,
    /// The package cache (HashMap of package name to package
    package_cache: RefCell<HashMap<String, Package>>,
}

impl Db {
    /// Create a new db instance
    pub(crate) fn new(
        name: DbName,
        handle: Rc<RefCell<Alpm>>,
        sig_level: SignatureLevel,
    ) -> Result<DbBase, ErrorKind> {
        if handle.database_exists(&name) {
            return Err(ErrorKind::DatabaseAlreadyExists(name));
        }

        Ok(DbBase::new_no_check_duplicates(name,
                                           sig_level,
                                           handle.database_path(),
                                           handle.database_extension()))
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
    pub(crate) fn new_no_check_duplicates(name: DbName,
                                          sig_level: SignatureLevel,
                                          database_path: impl AsRef<Path>,
                                          database_extension: impl AsRef<str>) -> DbBase {
        let path = name.path(database_path, database_extension);
        DbBase {
            name,
            sig_level,
            usage: DbUsage::ALL,
            servers: RefCell::new(HashSet::new()),
            path,
            package_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Get the name of the database
    #[inline]
    pub fn name(&self) -> &DbName {
        &self.name
    }

    /// Is this database a local database
    #[inline]
    pub fn is_local(&self) -> bool {
        self.name.is_local()
    }

    /// Is this database a sync database
    #[inline]
    pub fn is_sync(&self) -> bool {
        self.name.is_sync()
    }

    /// Get the path of this database.
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the registered servers for this database.
    #[inline]
    pub fn servers<'a>(&'a self) -> cell::Ref<'a, HashSet<Url>> {
        self.servers.borrow()
    }

    /// Add server
    pub fn add_server<'a>(&self, url: impl Into<UrlOrStr<'a>>) -> Result<(), Error>
    {
        // Convert to url
        let mut url = UrlOrStr::from(url).into_url()
            .context(ErrorKind::CannotAddServerToDatabase {
                url: format!("{}", s),
                database: self.base.name().to_owned(),
            })?;
        // Check db is sync
        if self.is_local() {
            return Err(err_msg("cannot add a server to a local database")
                .context(ErrorKind::CannotAddServerToDatabase {
                    url: format!("{}", url),
                    database: self.base.name().to_owned(),
                }).into());
        }
        // Check last char is a '/', otherwise we'll lose part of it when we add the database name
        match url.path().chars().next_back() {
            Some('/') => (),
            _ => {
                let mut path = url.path().to_owned();
                path.push('/');
                url.set_path(&path);
            },
        };
        debug!(r#"adding server with url "{}" from database "{}"."#, url, self.base.name);
        if ! self.base.servers.borrow_mut().insert(url.clone()) {
            warn!(r#"server with url "{}" was already present in database "{}"."#,
                  url, self.base.name);
        }
        Ok(())
    }

    /// Remove the server with the given url, if present
    pub fn remove_server<'a>(&mut self, url: impl Into<UrlOrStr<'a>>) -> Result<(), Error> {
        let mut url = UrlOrStr::from(url).into_url()
            .context(ErrorKind::CannotAddServerToDatabase {
                url: format!("{}", s),
                database: self.base.name().to_owned(),
            })?;
        debug!(r#"removing server with url "{}" from database "{}"."#,
               url, self.base.name);

        if ! self.base.servers.borrow_mut().remove(&url) {
            warn!(r#"server with url "{}" was not present in database "{}"."#,
                  url, self.base.name);
        }
        Ok(())
    }

    /// Remove all servers from this database.
    pub fn clear_servers(&mut self) {
        debug!(r#"removing all servers from database "{}"."#, self.base.name);
        self.base.servers.borrow_mut().clear()
    }

    /// Gets the database status
    pub fn status(&self) -> Result<DbStatus, Error> {
        // alpm checks path name, but we do this during construction.

        // check if database is missing
        let metadata = match fs::metadata(&self.path) {
            Err(ref e) if e.kind() == io::ErrorKind::NotFound =>
                return Ok(DbStatus::Missing),
            Err(e) =>
                return Err(e.context(ErrorKind::CannotQueryDatabase(
                    self.name.to_owned()
                )).into()),
            Ok(md) => md
        };

        self.is_valid(metadata).map(|valid| DbStatus::Exists { valid })
    }

    /// Validate the database.
    ///
    /// # Params
    ///  - `md` metadata for the database root
    ///  - `path` the path of the database root
    ///
    /// Returns true if the database is valid, false otherwise
    fn is_valid(&self, md: fs::Metadata) -> Result<bool, Error> {

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
            let version_path = self.path.join(LOCAL_DB_VERSION_FILE);
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
                    let mut read_dir = fs::read_dir(&self.path)
                        .context(ErrorKind::DatabaseVersion(self.name.to_owned()))?;
                    match read_dir.next() {
                        Some(_) => false,
                        None => {
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
            if ! md.is_file() {
                return Ok(false);
            }
            // todo check signature
            Ok(true)
        }
    }

    /// Synchronize this database with the remote servers.
    pub fn update(&self, mut force: bool) -> Result<(), Error> {
        use reqwest::header::IfModifiedSince;
        use reqwest::StatusCode;

        let name = match self.name {
            DbName(DbNameInner::Local) => {
                warn!("Updating the local database does nothing");
                return Ok(());
            },
            DbName(DbNameInner::Sync(ref name)) => name
        };
        debug!(r#"Updating remote database "{}"."#, name);
        // Force a reload when the db is invalid.
        match self.status()? {
            DbStatus::Exists { valid: true } => (),
            _ => {
                force = true;
            }
        };
        // todo this isn't how arch works - it gets the last update time from inside the db
        // somehow
        let modified = fs::metadata(self.path())
            .and_then(|md| md.modified())
            .ok();

        for server in self.servers.borrow().iter() {
            let filename = self.name.filename(self.handle.borrow().database_extension());
            let url = server.join(&filename).unwrap();
            debug!("Requesting update from {}", url);
            let mut request = handle.borrow().http_client.get(url);
            if let Some(modified) = modified {
                debug!("Database last updated at {:?}", modified);
                if ! force {
                    request.header(IfModifiedSince(modified.into()));
                }
            }
            let mut response = request.send().context(ErrorKind::UnexpectedReqwest)?;
            match response.status() {
                StatusCode::NotModified => {
                    // We're done
                    debug!("Server reports db not modified - finishing update.");
                    return Ok(());
                },
                StatusCode::Ok => (),
                code => {
                    warn!("Unexpected code {} while updating database {} - bailing",
                          code, self.name());
                    return Ok(());
                }
            }
            let mut db_file_opts = fs::OpenOptions::new();
            db_file_opts.write(true)
                .truncate(true);
            let mut db_file = db_file_opts.open(self.path())?;
            match db_file.try_lock_exclusive() {
                Ok(_) => Ok(()),
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    warn!("database {} is in use, blocking on request for exclusive access",
                          self.name());
                    db_file.lock_exclusive()
                },
                Err(e) => Err(e)
            }?;
            let len = response.copy_to(&mut db_file).context(ErrorKind::UnexpectedReqwest)?;
            debug!("Wrote {} bytes to db file {}", len, self.path().display());
        }
        Ok(())
    }

    /// Populate the package cache
    ///
    /// It is up to the caller to check that this database is local.
    fn populate_local(&self) -> Result<(), Error> {
        debug_assert!(self.is_local());
        for entry in fs::read_dir(self.path())? {
            let md = entry.metadata()?;
            if ! md.is_directory() {
                continue;
            }
        }
        Ok(())
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

    /// The filename of the database on disk
    ///
    /// This appends .db for sync databases, but not for local.
    fn filename(&self, ext: impl AsRef<str>) -> Cow<'static, str> {
        let ext = ext.as_ref();
        match &self.0 {
            &DbNameInner::Local => Cow::Borrowed(LOCAL_DB_NAME),
            &DbNameInner::Sync(ref name) => {
                let mut buf = String::with_capacity(name.len() + ext.len() + 1);
                buf.push_str(name);
                buf.push_str(".");
                buf.push_str(ext);
                Cow::Owned(buf)
            }
        }
    }

    /// Get the path for this database name
    ///
    /// Must supply the root database path from the alpm instance.
    pub(crate) fn path(&self, database_path: impl AsRef<Path>, ext: impl AsRef<str>) -> PathBuf {
        let database_path = database_path.as_ref();
        // path is
        //  - `$db_path SEP $name` for local
        //  - `$db_path SEP "sync" SEP $name "." $ext` for sync
        match &self.0 {
            &DbNameInner::Local => database_path.join(LOCAL_DB_NAME),
            &DbNameInner::Sync(ref name) => {
                let mut path = database_path.join(SYNC_DB_DIR);
                path.push(name);
                path.set_extension(ext.as_ref());
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

/// This structure only exists until `impl TryFrom<AsRef<str>> for Url` exists.
pub enum UrlOrStr<'a> {
    /// A url
    Url(Url),
    /// A borrowed string
    Str(&'a str),
}

impl<'a> UrlOrStr<'a> {
    fn into_url(self) -> Result<Url, impl Fail> {
        match self {
            Url(url) => Ok(url),
            Str(ref s) => s.parse(),
        }.context(format_err!(r#""{}" is not a valid url"#, s))
    }
}

impl From<Url> for UrlOrStr<'static> {
    fn from(url: Url) -> UrlOrStr<'static> {
        UrlOrStr::Url(url)
    }
}

impl<'a> From<&'a str> for UrlOrStr<'a> {
    fn from(s: &'a str) -> UrlOrStr<'a> {
        UrlOrStr::Str(s)
    }
}

impl<'a> From<&'a String> for UrlOrStr<'a> {
    fn from(s: &'a String) -> UrlOrStr<'a> {
        UrlOrStr::Str(s.as_ref())
    }
}

impl<'a> Display for UrlOrStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UrlOrStr::Url(ref url) => Display::fmt(url, f),
            UrlOrStr::Str(ref s) => Display::fmt(s, f)
        }
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
        let ext = "db";

        let tests = vec![
            ("local", "/var/lib/pacman/local"),
            ("sync1", "/var/lib/pacman/sync/sync1.db"),
        ];
        for (db_name, target) in tests {
            let db_name = DbName::new(db_name).unwrap();
            let target = Path::new(target);
            assert_eq!(db_name.path(&base_path, &ext), target);
            assert_eq!(db_name.path(&base_path2, &ext), target);
        }
    }
}
