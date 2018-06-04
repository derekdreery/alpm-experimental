//! A library to manipulate a system managed by the Alpm (Arch Linux Package Manager).
//!

#![feature(nll)]
#![feature(str_escape)]

extern crate atoi;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;
//extern crate fs2;
extern crate itertools;
extern crate lockfile;
#[macro_use]
extern crate log;
#[macro_use] // pollute away :(
extern crate nom;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tempfile;

pub mod alpm_desc;
mod error;
mod util;

pub mod db;

pub use error::{Error, ErrorKind};

use db::{Db, DbBase, DbName, SignatureLevel};
use util::check_valid_directory;

use failure::{Fail, ResultExt};
use lockfile::Lockfile;

use std::io;
use std::path::{Path, PathBuf};

/// The name of the lockfile (hard-coded).
pub const LOCKFILE: &str = "db.lck";
/// The name of the local database.
pub const LOCAL_DB_NAME: &str = "local";
/// The name of the directory for sync databases.
pub const SYNC_DB_DIR: &str = "sync";
/// The extension of the directory for sync databases.
pub const SYNC_DB_EXT: &str = "db";

/// Handle to an alpm instance. Uses a lockfile to prevent concurrent access to the
/// same db.
pub struct Alpm {
    /// The local package database
    local_database: DbBase,
    /// A list of all sync databases
    sync_databases: Vec<DbBase>,
    /// Managed filesystem root (normally this will be "/")
    root_path: PathBuf,
    /// The path of the alpm package database
    database_path: PathBuf,
    /// The lockfile, preventing multiple processes
    /// interacting with the database concurrently.
    lockfile: Lockfile,
    /// Path to the directory where gpg files are stored
    gpg_path: PathBuf,
    /// List of paths to the cache directories
    cache_dirs_paths: Vec<PathBuf>,
    /// List of paths to the hook directories
    hook_dirs_paths: Vec<PathBuf>,
    /// List of paths that may be overwritten
    overwrite_file_paths: Vec<PathBuf>,
    /// Cached reqwest client, for speed
    http_client: reqwest::Client,
}

impl Alpm {
    /// Create a builder for a new alpm instance.
    ///
    /// # Examples
    ///
    /// Create a new instance using the defaults
    /// ```
    /// # use alpm::Alpm;
    /// let alpm = Alpm::new().build();
    /// ```
    ///
    /// Create a new instance for a chroot environment
    /// ```
    /// # use alpm::Alpm;
    /// let alpm = Alpm::new()
    ///     .with_root_path("/my/chroot")
    ///     .build();
    /// ```
    pub fn new() -> AlpmBuilder {
        Default::default()
    }

    /// Register a new sync database
    ///
    /// The name must not match `LOCAL_DB_NAME`.
    pub fn register_sync_database<'a>(
        &'a mut self,
        name: impl AsRef<str>,
    ) -> Result<Db<'a>, Error> {
        let name = name.as_ref();
        // If we've already registered the database, just return it
        if let Some(db) = self.sync_databases.iter().find(|&db| db.name() == name) {
            warn!(r#"database "{}" already registered"#, name);
            return Ok(Db::new(db, self));
        }
        let base = DbBase::new_sync(name, self, SignatureLevel::default())?;
        let db_idx = self.sync_databases.len();
        self.sync_databases.push(base);
        Ok(Db::new(&self.sync_databases[db_idx], self))
    }

    /// Are there any databases already registered with the given name
    pub fn database_exists(&self, name: impl AsRef<str>) -> bool {
        let name = name.as_ref();
        if name == LOCAL_DB_NAME {
            return true;
        }
        self.sync_databases.iter().any(|db| db.name() == name)
    }

    /// Unregister a sync database.
    ///
    /// Database is left on the filesystem and will not be touched after this is called.
    pub fn unregister_sync_database(&mut self, name: impl AsRef<str>) {
        let name = name.as_ref();
        if let Some(idx) = self.sync_databases.iter().position(|db| db.name() == name) {
            self.sync_databases.remove(idx);
        } else {
            warn!("could not find a database with name \"{}\"", name);
        }
    }

    /// Get the local database for this alpm instance.
    pub fn local_database<'a>(&'a self) -> Db<'a> {
        Db::new(&self.local_database, self)
    }

    /// Get a sync database with the given name for this alpm instance.
    pub fn sync_database<'a>(&'a self, name: impl AsRef<str>) -> Option<Db<'a>> {
        self.sync_databases.iter()
            .find(|&db| db.name().as_str() == name.as_ref())
            .map(|db| Db::new(db, self))
    }
}

/// Builder-pattern constructor for the Alpm struct.
///
/// Use `Alpm::new` to get an `AlpmBuilder`, use `AlpmBuilder::build` to get an `Alpm` instance.
///
/// See `Alpm` struct for field documentation.
pub struct AlpmBuilder {
    /// Root path for filesystem. Defaults to "/" on non-windows, "C:\" on windows.
    root_path: Option<PathBuf>,
    /// Path for the alpm database. Defaults to "$root/var/lib/pacman"
    database_path: Option<PathBuf>,
    /// todo
    gpg_path: Option<PathBuf>,
}

impl Default for AlpmBuilder {
    fn default() -> Self {
        AlpmBuilder {
            root_path: None,
            database_path: None,
            gpg_path: None,
        }
    }
}

impl AlpmBuilder {
    /// Use custom root path.
    pub fn with_root_path(mut self, root_path: impl AsRef<Path>) -> Self {
        self.root_path = Some(root_path.as_ref().to_owned());
        self
    }

    /// Use custom database path
    pub fn with_database_path(mut self, database_path: impl AsRef<Path>) -> Self {
        self.database_path = Some(database_path.as_ref().to_owned());
        self
    }

    /// Use custom gpg location
    pub fn with_gpg_path(mut self, gpg_path: impl AsRef<Path>) -> Self {
        self.gpg_path = Some(gpg_path.as_ref().to_owned());
        self
    }

    /// Build the alpm instance.
    pub fn build(self) -> Result<Alpm, Error> {
        #[cfg(windows)]
        let root_path = self.root_path.unwrap_or("C:\\".into());
        #[cfg(not(windows))]
        let root_path = self.root_path.unwrap_or("/".into());
        debug!("root path: {}", root_path.display());
        check_valid_directory(&root_path).context(ErrorKind::BadRootPath(root_path.clone()))?;

        // todo sensible default database path on windows
        let database_path = match self.database_path {
            Some(path) => path,
            None => {
                let mut db_path = root_path.clone();
                db_path.push("var/lib/pacman");
                db_path
            }
        };
        debug!("database path: {}", database_path.display());
        check_valid_directory(&database_path)
            .context(ErrorKind::BadDatabasePath(database_path.clone()))?;

        let sync_db_path = database_path.join(SYNC_DB_DIR);
        debug!("sync database path: {}", sync_db_path.display());
        check_valid_directory(&sync_db_path)
            .context(ErrorKind::BadSyncDatabasePath(sync_db_path.clone()))?;

        // todo
        let gpg_path = root_path.clone();
        debug!("gpg path: {}", gpg_path.display());

        let lockfile_path = database_path.join(LOCKFILE);
        debug!("lockfile path: {}", lockfile_path.display());

        let lockfile = Lockfile::create(&lockfile_path).map_err(|e| {
            let kind = e.kind();
            if kind == io::ErrorKind::AlreadyExists {
                e.context(ErrorKind::LockAlreadyExists(lockfile_path.clone()))
            } else {
                e.context(ErrorKind::CannotAcquireLock(lockfile_path.clone()))
            }
        })?;

        let alpm = Alpm {
            local_database: DbBase::new_no_check_duplicates(
                DbName::LOCAL.clone(),
                SignatureLevel::Default,
            ),
            sync_databases: Vec::new(),
            root_path,
            database_path,
            lockfile,
            gpg_path,
            cache_dirs_paths: Vec::new(),
            hook_dirs_paths: Vec::new(),
            overwrite_file_paths: Vec::new(),
            http_client: reqwest::Client::new(),
        };
        Ok(alpm)
    }
}
