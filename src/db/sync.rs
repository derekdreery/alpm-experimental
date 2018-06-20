use std::borrow::Cow;
use std::cell::{Ref, RefMut, RefCell};
use std::collections::{HashSet, HashMap, hash_set};
use std::convert::TryInto;
use std::cmp;
use std::fmt::{self, Display};
use std::fs;
use std::io::{self, Read, Write};
use std::ops::Deref;
use std::path::{self, Path, PathBuf};
use std::rc::{Rc, Weak as WeakRc};

use db::{SYNC_DB_DIR, LOCAL_DB_NAME, Database, DbStatus, DbUsage, SignatureLevel};
use error::{ErrorKind, Error};
use package::Package;
use Handle;
use util::UrlOrStr;

use atoi::atoi;
use failure::{Fail, ResultExt, err_msg};
use fs2::FileExt;
use reqwest::{self, Url};

#[derive(Debug)]
pub struct SyncDatabase {
    // Cache name and path
    name: String,
    path: PathBuf,
    inner: WeakRc<RefCell<SyncDatabaseInner>>
}

impl SyncDatabase {

    #[inline]
    pub(crate) fn new(db: &Rc<RefCell<SyncDatabaseInner>>, name: String, path: PathBuf) -> Self {
        SyncDatabase { inner: Rc::downgrade(db), name, path }
    }

    /// Get a copy of the registered servers for this database.
    #[inline]
    pub fn servers<'a>(&'a self) -> Result<Vec<Url>, Error> {
        Ok(self.upgrade()?.borrow_mut().servers.iter().map(|url| url.clone()).collect())
    }

    /// Add server
    #[inline]
    pub fn add_server<U>(&mut self, url: U) -> Result<(), Error>
    where
        UrlOrStr: From<U>
    {
        self.upgrade()?.borrow_mut().add_server(url)
    }

    /// Remove the server with the given url, if present
    pub fn remove_server<U>(&mut self, url: U) -> Result<(), Error>
        where
            UrlOrStr: From<U>
    {
        self.upgrade()?.borrow_mut().remove_server(url)
    }

    /// Remove all servers from this database.
    pub fn clear_servers(&self) -> Result<(), Error> {
        let db = self.upgrade()?;
        db.borrow_mut().clear_servers();
        Ok(())
    }

    fn upgrade(&self) -> Result<Rc<RefCell<SyncDatabaseInner>>, Error> {
        WeakRc::upgrade(&self.inner).ok_or(ErrorKind::UseAfterDrop.into())
    }
}

impl Database for SyncDatabase {
    /// Get the name of the database
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    /// Get the path of this database.
    #[inline]
    fn path(&self) -> &Path {
        &self.path
    }

    /// Get the status of this database.
    fn status(&self) -> Result<DbStatus, Error> {
        self.upgrade()?.borrow().status()
    }

    /// Synchronize the database with any external sources.
    fn synchronize(&self, force: bool) -> Result<(), Error> {
        self.upgrade()?.borrow_mut().synchronize(force)
    }

    /// Get a package in this database
    fn package(&self, name: &str) -> Result<Rc<Package>, Error> {
        unimplemented!();
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
    /// The package cache (HashMap of package name to package
    package_cache: HashMap<String, Package>,
}

impl SyncDatabaseInner {

    /// Create a new sync db instance
    ///
    /// The name of this database must not match LOCAL_DB_NAME
    ///
    /// # Panics
    ///
    /// This function panics if a SyncDatabase already exists with the given name
    pub(crate) fn new(handle: Rc<RefCell<Handle>>,
                      name: SyncDbName,
                      sig_level: SignatureLevel) -> SyncDatabaseInner
    {
        let handle_ref = handle.borrow();
        // This is the caller's responsibility.
        assert!(! handle_ref.sync_database_registered(&name),
                "internal error - database already exists");
        let db_filename = name.filename(&handle_ref.database_extension);
        let path = handle_ref.database_path.join(db_filename);
        drop(handle_ref);
        SyncDatabaseInner {
            handle: Rc::downgrade(&handle),
            name,
            sig_level,
            usage: DbUsage::ALL,
            servers: HashSet::new(),
            path,
            package_cache: HashMap::new(),
        }
    }


    /// Get the registered servers for this database.
    #[inline]
    fn servers(&self) -> &HashSet<Url> {
        &self.servers
    }

    /// Add server
    pub fn add_server<U>(&mut self, url: U) -> Result<(), Error>
        where
            UrlOrStr: From<U>
    {
        // Convert to url
        let mut url = UrlOrStr::from(url).into_url()
            .map_err(|(s, e)| {
                e.context(ErrorKind::CannotAddServerToDatabase {
                    url: format!("{}", s),
                    database: self.name.to_string(),
                })
            })?;
        // Check last char is a '/', otherwise we'll lose part of it when we add the database name
        match url.path().chars().next_back() {
            Some('/') => (),
            _ => {
                let mut path = url.path().to_owned();
                path.push('/');
                url.set_path(&path);
            },
        };
        debug!(r#"adding server with url "{}" from database "{}"."#, url, self.name);
        if ! self.servers.insert(url.clone()) {
            warn!(r#"server with url "{}" was already present in database "{}"."#,
                  url, self.name);
        }
        Ok(())
    }

    /// Remove the server with the given url, if present
    pub fn remove_server<U>(&mut self, url: U) -> Result<(), Error>
        where
            UrlOrStr: From<U>
    {
        let url = UrlOrStr::from(url).into_url()
            .map_err(|(s, e)| {
                e.context(ErrorKind::CannotAddServerToDatabase {
                    url: format!("{}", s),
                    database: self.name.to_string(),
                })
            })?;
        debug!(r#"removing server with url "{}" from database "{}"."#,
               url, self.name);

        if ! self.servers.remove(&url) {
            warn!(r#"server with url "{}" was not present in database "{}"."#,
                  url, self.name);
        }
        Ok(())
    }

    /// Remove all servers from this database.
    pub fn clear_servers(&mut self) {
        debug!(r#"removing all servers from database "{}"."#, self.name);
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

        if ! md.is_file() {
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
            Err(ref e) if e.kind() == io::ErrorKind::NotFound =>
                return Ok(DbStatus::Missing),
            Err(e) => return Err(e.into()),
            Ok(md) => md
        };

        Ok(DbStatus::Exists { valid: self.is_valid(metadata) })
    }

    /// Synchronize the database with any external sources.
    fn synchronize(&mut self, force: bool) -> Result<(), Error> {
        use reqwest::header::IfModifiedSince;
        use reqwest::StatusCode;

        debug!(r#"Updating sync database "{}"."#, self.name);

        let handle = self.get_handle()?;
        let handle_ref = handle.borrow();

        // Force a reload when the db is invalid.
        let mut force = force;
        match self.status()? {
            DbStatus::Exists { valid: true } => (),
            _ => { force = true; }
        };

        // todo this possibly isn't how arch works - it may get the last update time from inside
        // the db somehow
        let modified = fs::metadata(&self.path)
            .and_then(|md| md.modified())
            .ok();

        for server in self.servers.iter() {
            let filename = self.name.filename(&handle_ref.database_extension);
            let url = server.join(&filename).unwrap();
            debug!("Requesting update from {}", url);
            let mut request = handle_ref.http_client.get(url);
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
                          code, self.name);
                    return Ok(());
                }
            }
            let mut db_file_opts = fs::OpenOptions::new();
            db_file_opts
                .create(true)
                .write(true)
                .truncate(true);
            let mut db_file = db_file_opts.open(&*self.path)?;
            match db_file.try_lock_exclusive() {
                Ok(_) => Ok(()),
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    warn!("database {} is in use, blocking on request for exclusive access",
                          self.name);
                    db_file.lock_exclusive()
                },
                Err(e) => Err(e)
            }?;
            let len = response.copy_to(&mut db_file).context(ErrorKind::UnexpectedReqwest)?;
            debug!("Wrote {} bytes to db file {}", len, self.path.display());
        }
        Ok(())
    }

    /// Get the packages in this database
    fn packages(&self) -> &HashMap<String, Package> {
        unimplemented!();
    }

    /// Fetches an alpm handle and maps failure to an error
    fn get_handle(&self) -> Result<Rc<RefCell<Handle>>, Error> {
        self.handle.upgrade().ok_or(ErrorKind::UseAfterDrop.into())
    }
}

/// The name (and implied type) of an alpm database.
///
/// Valid database names do not contain path separators (on any OS), or the dot char ('.').
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct SyncDbName(String);

impl SyncDbName {
    /// Create a new valid SyncDbName.
    ///
    /// Returns an error if the name isn't a valid directory name.
    pub(crate) fn new(name: impl AsRef<str>) -> Result<SyncDbName, ErrorKind> {
        let name = name.as_ref();
        let db_name = match name {
            name if name == LOCAL_DB_NAME =>
                return Err(ErrorKind::InvalidDatabaseName(name.to_owned())),
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
    pub(crate) fn path(&self, database_path: impl AsRef<Path>, ext: impl AsRef<str>) -> PathBuf {
        let database_path = database_path.as_ref();
        //  database path `$db_path SEP "sync" SEP $name "." $ext`
        let mut path = database_path.join(SYNC_DB_DIR);
        path.push(&self.0);
        path.set_extension(ext.as_ref());
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
