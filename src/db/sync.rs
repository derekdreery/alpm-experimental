//! Remote databases (a.k.a. "sync databases")
//!
//! Sync databases are the same as the local database, except that they don't have the `file` and
//! `mtree` files, and they are `tar`d and `gzipped` up.

use std::borrow::Cow;
use std::cell::RefCell;
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{self, Path, PathBuf};
use std::rc::{Rc, Weak as WeakRc};

use crate::db::{
    Database, DbStatus, DbUsage, SignatureLevel, DEFAULT_SYNC_DB_EXT, LOCAL_DB_NAME, SYNC_DB_DIR,
};
use crate::error::{Error, ErrorContext, ErrorKind};
use crate::util::UrlOrStr;
use crate::Handle;

use fs2::FileExt;
use libflate::gzip;
use reqwest::Url;

pub use self::package::SyncPackage;

mod package;

const HTTP_DATE_FORMAT: &str = "%a, %d %b %Y %T GMT";

/// A sync database of available packages.
#[derive(Debug, Clone)]
pub struct SyncDatabase {
    // Cache name and path
    name: String,
    inner: Rc<RefCell<SyncDatabaseInner>>,
}

impl SyncDatabase {
    #[inline]
    pub(crate) fn new(inner: Rc<RefCell<SyncDatabaseInner>>, name: String) -> Self {
        SyncDatabase { inner, name }
    }

    /// Get a copy of the registered servers for this database.
    #[inline]
    pub fn servers<'a>(&'a self) -> Result<Vec<Url>, Error> {
        Ok(self
            .inner
            .borrow_mut()
            .servers
            .iter()
            .map(|url| url.clone())
            .collect())
    }

    /// Add server
    #[inline]
    pub fn add_server<U>(&mut self, url: U) -> Result<(), Error>
    where
        UrlOrStr: From<U>,
    {
        self.inner.borrow_mut().add_server(url)
    }

    /// Remove the server with the given url, if present
    pub fn remove_server<U>(&mut self, url: U) -> Result<(), Error>
    where
        UrlOrStr: From<U>,
    {
        self.inner.borrow_mut().remove_server(url)
    }

    /// Remove all servers from this database.
    pub fn clear_servers(&self) {
        self.inner.borrow_mut().clear_servers();
    }

    /// Synchronize the database with any external sources.
    pub fn synchronize(&self, force: bool) -> Result<(), Error> {
        self.inner.borrow_mut().synchronize(force)
    }
}

impl Database for SyncDatabase {
    type Pkg = Rc<SyncPackage>;

    fn name(&self) -> &str {
        &self.name
    }

    fn path(&self) -> PathBuf {
        self.inner.borrow().path.clone()
    }

    fn status(&self) -> Result<DbStatus, Error> {
        self.inner.borrow().status()
    }

    fn count(&self) -> usize {
        unimplemented!()
    }

    fn package(&self, name: impl AsRef<str>, version: impl AsRef<str>) -> Result<Self::Pkg, Error> {
        use crate::package::Package;

        let name = name.as_ref();
        let version = version.as_ref();
        let db = self.inner.borrow();
        let package = db
            .package_cache
            .get(&Cow::Borrowed(name))
            .ok_or(ErrorKind::InvalidLocalPackage(name.to_owned()))?;
        if version != package.version() {
            return Err(ErrorKind::InvalidLocalPackage(name.to_owned()))?;
        }
        Ok(package.clone())
    }

    fn package_latest<Str>(&self, name: Str) -> Result<Self::Pkg, Error>
    where
        Str: AsRef<str>,
    {
        let name = name.as_ref();
        let package = self
            .inner
            .borrow()
            .package_cache
            .get(&Cow::Borrowed(name))
            .ok_or(ErrorKind::InvalidLocalPackage(name.to_owned()))?
            .clone();
        Ok(package)
    }

    fn packages<E, F>(&self, mut f: F) -> Result<(), E>
    where
        F: FnMut(Self::Pkg) -> Result<(), E>,
        E: From<Error>,
    {
        let db = self.inner.borrow();
        for package in db.package_cache.values() {
            f(package.clone())?;
        }
        Ok(())
    }
}

/// A package database.
#[derive(Debug)]
pub struct SyncDatabaseInner {
    /// Handle to the alpm instance
    handle: WeakRc<RefCell<Handle>>,
    /// The name of the database, also used to construct the database path.
    name: SyncDbName,
    /// The level of signature verification required to accept packages
    sig_level: SignatureLevel,
    /// Which operations this database will be used for.
    usage: DbUsage,
    /// A list of servers for this database
    servers: HashSet<Url>,
    /// The database path.
    pub path: PathBuf,
    /// The package cache (HashMap of package name to package)
    // Unlike in LocalDatabaseInner we don't have a version, since there is only one version of any
    // package in a sync repository.
    package_cache: HashMap<Cow<'static, str>, Rc<SyncPackage>>,
    /// Count of the number of packages (cached)
    package_count: usize,
}
impl SyncDatabaseInner {
    /// Create a new sync db instance
    ///
    /// The name of this database must not match LOCAL_DB_NAME
    ///
    /// # Panics
    ///
    /// This function panics if a SyncDatabase already exists with the given name
    pub(crate) fn new(
        handle: Rc<RefCell<Handle>>,
        name: SyncDbName,
        sig_level: SignatureLevel,
    ) -> SyncDatabaseInner {
        let handle_ref = handle.borrow();
        // This is the caller's responsibility.
        assert!(
            !handle_ref.sync_database_registered(&name),
            "internal error - database already exists"
        );
        let path = name.path(&handle_ref.database_path);
        drop(handle_ref);
        let mut db = SyncDatabaseInner {
            handle: Rc::downgrade(&handle),
            name,
            sig_level,
            usage: DbUsage::ALL,
            servers: HashSet::new(),
            path,
            package_cache: HashMap::new(),
            package_count: 0,
        };
        db.populate_package_cache().unwrap();
        db
    }

    /// Add server
    pub fn add_server<U>(&mut self, url: U) -> Result<(), Error>
    where
        UrlOrStr: From<U>,
    {
        // Convert to url
        let mut url = UrlOrStr::from(url).into_url().map_err(|(s, e)| {
            Error::from(ErrorKind::CannotAddServerToDatabase {
                url: format!("{}", s),
                database: self.name.to_string(),
            })
            .with_source(e)
        })?;
        // Check last char is a '/', otherwise we'll lose part of it when we add the database name
        match url.path().chars().next_back() {
            Some('/') => (),
            _ => {
                let mut path = url.path().to_owned();
                path.push('/');
                url.set_path(&path);
            }
        };
        log::debug!(
            r#"adding server with url "{}" from database "{}"."#,
            url,
            self.name
        );
        if !self.servers.insert(url.clone()) {
            log::warn!(
                r#"server with url "{}" was already present in database "{}"."#,
                url,
                self.name
            );
        }
        Ok(())
    }

    /// Remove the server with the given url, if present
    pub fn remove_server<U>(&mut self, url: U) -> Result<(), Error>
    where
        UrlOrStr: From<U>,
    {
        let url = UrlOrStr::from(url).into_url().map_err(|(s, e)| {
            Error::from(ErrorKind::CannotAddServerToDatabase {
                url: format!("{}", s),
                database: self.name.to_string(),
            })
            .with_source(e)
        })?;
        log::debug!(
            r#"removing server with url "{}" from database "{}"."#,
            url,
            self.name
        );

        if !self.servers.remove(&url) {
            log::warn!(
                r#"server with url "{}" was not present in database "{}"."#,
                url,
                self.name
            );
        }
        Ok(())
    }

    /// Remove all servers from this database.
    pub fn clear_servers(&mut self) {
        log::debug!(r#"removing all servers from database "{}"."#, self.name);
        self.servers.clear()
    }

    /// Validate the database.
    ///
    /// # Params
    ///  - `md` metadata for the database root
    ///  - `path` the path of the database root
    ///
    /// Returns true if the database is valid, false otherwise
    fn is_valid(&self, md: fs::Metadata) -> bool {
        if !md.is_file() {
            return false;
        }
        // todo check signature
        true
    }

    /// Get the status of this database.
    fn status(&self) -> Result<DbStatus, Error> {
        // alpm checks path name, but we do this during construction.

        // check if database is missing
        let metadata = match fs::metadata(&self.path) {
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => return Ok(DbStatus::Missing),
            Err(e) => return Err(e.into()),
            Ok(md) => md,
        };

        Ok(if self.is_valid(metadata) {
            DbStatus::Valid
        } else {
            DbStatus::Invalid
        })
    }

    /// Synchronize the database with any external sources.
    fn synchronize(&mut self, mut force: bool) -> Result<(), Error> {
        use chrono::{DateTime, Utc};
        use reqwest::header::IF_MODIFIED_SINCE;
        use reqwest::StatusCode;
        use std::time::SystemTime;

        log::debug!(r#"Updating sync database "{}"."#, self.name);

        let handle = self.get_handle()?;
        let handle_ref = handle.borrow();

        // Force a reload when the db is invalid.
        match self.status()? {
            DbStatus::Valid => (),
            DbStatus::Invalid | DbStatus::Missing => {
                force = true;
            }
        };

        // todo this possibly isn't how arch works - it may get the last update time from inside
        // the db somehow
        let modified = fs::metadata(&self.path).and_then(|md| md.modified()).ok();

        for server in self.servers.iter() {
            let filename = self.name.filename(&handle_ref.database_extension);
            let url = server.join(&filename).unwrap();
            log::debug!("Requesting update from {}", url);
            let mut request = handle_ref.http_client.get(url);
            if let Some(modified) = modified {
                log::debug!("Database last updated at {:?}", modified);
                if !force {
                    // Set If-Modified-Since header to avoid unnecessary download.
                    let modified = <DateTime<Utc> as From<SystemTime>>::from(modified);
                    let modified = format!("{}", modified.format(HTTP_DATE_FORMAT));
                    request = request.header(IF_MODIFIED_SINCE, modified);
                }
            }
            let mut response = request.send().context(ErrorKind::UnexpectedReqwest)?;
            match response.status() {
                StatusCode::NOT_MODIFIED => {
                    // We're done
                    log::debug!("Server reports db not modified - finishing update.");
                    return Ok(());
                }
                StatusCode::OK => (),
                code => {
                    log::warn!(
                        "Unexpected code {} while updating database {} - bailing",
                        code,
                        self.name
                    );
                    return Ok(());
                }
            }
            let mut db_file_opts = fs::OpenOptions::new();
            db_file_opts.create(true).write(true).truncate(true);
            let mut db_file = db_file_opts.open(&*self.path)?;
            match db_file.try_lock_exclusive() {
                Ok(_) => Ok(()),
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    log::warn!(
                        "database {} is in use, blocking on request for exclusive access",
                        self.name
                    );
                    db_file.lock_exclusive()
                }
                Err(e) => Err(e),
            }?;
            let len = response
                .copy_to(&mut db_file)
                .context(ErrorKind::UnexpectedReqwest)?;
            log::debug!("Wrote {} bytes to db file {}", len, self.path.display());
        }
        Ok(())
    }

    /// Fetches an alpm handle and maps failure to an error
    fn get_handle(&self) -> Result<Rc<RefCell<Handle>>, Error> {
        self.handle.upgrade().ok_or(ErrorKind::UseAfterDrop.into())
    }

    /// Load all packags into the cache, and validate the database
    pub(crate) fn populate_package_cache(&mut self) -> Result<(), Error> {
        use std::io::Read;

        log::info!("Getting cache from {}", self.path.display());
        // Times like this you wish you were in haskell
        let mut reader = tar::Archive::new(gzip::Decoder::new(io::BufReader::new(
            fs::File::open(&self.path)?,
        ))?);

        if !self.package_cache.is_empty() || self.package_count != 0 {
            panic!("populate_package_cache should only be called once on database cration");
        }

        for entry in reader.entries()? {
            let mut entry = entry?;

            let path = entry.path()?;
            let file_name = match path.file_name() {
                Some(p) if p == "desc" => path
                    .parent()
                    .and_then(|parent| parent.file_name())
                    .expect("TODO handle malformed db archive")
                    .to_str()
                    .expect("TODO handle non-utf8 package name")
                    .to_owned(),
                _ => continue,
            };
            let (name, version) = super::split_package_dirname(&file_name)
                .ok_or(ErrorKind::InvalidSyncPackage(file_name.to_owned()))?;
            log::debug!(r#"found "{}", version: "{}""#, name, version);

            // Get contents of desc file
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents)?;
            let contents = String::from_utf8(contents)
                .context(ErrorKind::InvalidSyncPackage(name.to_owned()))?;
            let package = SyncPackage::from_parts(&contents, &name, &version)?;

            if self
                .package_cache
                .insert(Cow::Owned(name.to_owned()), Rc::new(package))
                .is_some()
            {
                panic!(
                    "internal error - there should only ever be 1 package with a given name \
                     in a sync database"
                );
            }
            self.package_count += 1;
        }
        Ok(())
    }
}

/// The name (and implied type) of an alpm database.
///
/// Valid database names do not contain path separators, or the dot char ('.').
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct SyncDbName(String);

impl SyncDbName {
    /// Create a new valid SyncDbName.
    ///
    /// Returns an error if the name isn't a valid directory name.
    pub(crate) fn new(name: impl AsRef<str>) -> Result<SyncDbName, ErrorKind> {
        let name = name.as_ref();
        let db_name = match name {
            name if name == LOCAL_DB_NAME => {
                return Err(ErrorKind::InvalidDatabaseName(name.to_owned()))
            }
            name => SyncDbName(name.to_owned()),
        };
        if db_name.is_valid() {
            Ok(db_name)
        } else {
            Err(ErrorKind::InvalidDatabaseName(db_name.0))
        }
    }

    /// The filename of the database on disk
    ///
    /// This appends .db for sync databases. It is a String because it is used in Urls as well as
    /// on the fs.
    fn filename(&self, ext: impl AsRef<str>) -> String {
        let ext = ext.as_ref();
        let mut buf = String::with_capacity(self.0.len() + ext.len() + 1);
        buf.push_str(&self.0);
        buf.push_str(".");
        buf.push_str(ext);
        buf.into()
    }

    /// Get the path for this database name
    ///
    /// Must supply the root database path from the alpm instance.
    pub(crate) fn path(&self, database_path: impl AsRef<Path>) -> PathBuf {
        let database_path = database_path.as_ref();
        //  database path `$db_path SEP "sync" SEP $name "." $ext`
        let mut path = database_path.join(SYNC_DB_DIR);
        path.push(&self.0);
        path.set_extension(DEFAULT_SYNC_DB_EXT);
        path
    }

    /// Is the string a valid sync database name?
    ///
    /// Fails if the name contains path separators (for any OS environment) or dot ('.')
    pub fn is_valid(&self) -> bool {
        for ch in self.0.chars() {
            if path::is_separator(ch) || ch == '.' || ch == '\\' || ch == '/' {
                return false;
            }
        }
        true
    }
}

impl fmt::Display for SyncDbName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SyncDbName {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<SyncDbName> for String {
    #[inline]
    fn from(db_name: SyncDbName) -> String {
        db_name.0
    }
}

impl cmp::PartialEq<str> for SyncDbName {
    fn eq(&self, rhs: &str) -> bool {
        cmp::PartialEq::eq(self.as_ref(), rhs)
    }

    fn ne(&self, rhs: &str) -> bool {
        cmp::PartialEq::ne(self.as_ref(), rhs)
    }
}

impl cmp::PartialEq<SyncDbName> for str {
    fn eq(&self, rhs: &SyncDbName) -> bool {
        cmp::PartialEq::eq(self, rhs.as_ref())
    }

    fn ne(&self, rhs: &SyncDbName) -> bool {
        cmp::PartialEq::ne(self, rhs.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_name() {
        assert_eq!(
            SyncDbName::new("name_of_db").unwrap(),
            SyncDbName("name_of_db".into())
        );
        assert!(SyncDbName::new("bad/name").is_err());
        assert!(SyncDbName::new("bad\\name").is_err());
        assert!(SyncDbName::new("bad.name").is_err());
    }
}
