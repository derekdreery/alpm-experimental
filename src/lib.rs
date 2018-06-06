//! A library to manipulate a system managed by the Alpm (Arch Linux Package Manager).
//!

#![feature(nll)]
#![feature(str_escape)]
#![feature(try_from)]

extern crate atoi;
#[macro_use]
extern crate bitflags;
extern crate chrono;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;
//extern crate fs2;
extern crate gpgme;
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
#[cfg(not(windows))]
extern crate uname;

pub mod alpm_desc;
mod error;
mod signing;
mod util;

pub mod db;

pub use error::{Error, ErrorKind};

pub use db::Db;
use db::{DbBase, DbName, SignatureLevel};

use failure::{Fail, ResultExt};
use lockfile::Lockfile;
use uname::uname;

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::ops::Deref;

/// The name of the lockfile (hard-coded).
pub const LOCKFILE: &str = "db.lck";
/// The name of the local database.
pub const LOCAL_DB_NAME: &str = "local";
/// The name of the directory for sync databases.
pub const SYNC_DB_DIR: &str = "sync";
/// The extension of the directory for sync databases.
const DEFAULT_SYNC_DB_EXT: &str = "db";

/// Handle to an alpm instance. Uses a lockfile to prevent concurrent access to the
/// same db.
pub struct Alpm {
    /// The local package database
    local_database: DbBase,
    /// A list of all sync databases
    sync_databases: HashMap<DbName, DbBase>,
    /// Managed filesystem root (normally this will be "/")
    root_path: PathBuf,
    /// The path of the alpm package database
    database_path: PathBuf,
    /// The extension to use for sync databases
    database_extension: String,
    /// The lockfile, preventing multiple processes
    /// interacting with the database concurrently.
    #[allow(unused)]
    lockfile: Lockfile,
    /// Path to the directory where gpg files are stored
    gpg_path: PathBuf,
    /// List of paths to the cache directories
    cache_dirs_paths: HashSet<PathBuf>,
    /// List of paths to the hook directories
    hook_dirs_paths: HashSet<PathBuf>,
    /// List of paths that may be overwritten
    overwrite_file_paths: HashSet<PathBuf>,
    /// List of packages not to upgrade.
    packages_no_upgrade: HashSet<String>,
    /// List of packages not to extract.
    packages_no_extract: HashSet<String>,
    /// List of packages to ignore.
    packages_ignore: HashSet<String>,
    /// List of groups to ignore.
    groups_ignore: HashSet<String>,
    /// List of virtual packages used to satisfy dependencies.
    packages_assume_installed: HashSet<String>,
    /// The architecture of the packages to be installed.
    arch: String,
    /// Download deltas if possible; a ratio value.
    delta_ratio: f64,
    /// Whether to check free disk space before installing.
    check_space: bool,
    // database_extension: String,
    ///// The signature veritification level to use when databases or packages inherit.
    // signature_level: SignatureLevel,
    // local_files_signature_level: SignatureLevel,
    // remote_files_signature_level: SignatureLevel,
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
        let name = DbName::new(name.as_ref())?;
        if self.sync_databases.contains_key(&name) {
            warn!(r#"database "{}" already registered"#, name);
        } else {
            let new_db = DbBase::new_sync(name.clone(), self, SignatureLevel::default())?;
            self.sync_databases.insert(name.clone(), new_db);
        }
        Ok(Db::new(self.sync_databases.get(&name).unwrap(), self))
    }

    /// Are there any databases already registered with the given name
    pub fn database_exists(&self, name: &DbName) -> bool {
        self.sync_databases.contains_key(name)
    }

    /// Unregister a sync database.
    ///
    /// Database is left on the filesystem and will not be touched after this is called.
    pub fn unregister_sync_database(&mut self, name: &DbName) {
        if ! self.sync_databases.remove(name).is_none() {
            warn!("could not find a database with name \"{}\"", name);
        }
    }

    pub fn unregister_all_sync_databases(&mut self) {
        self.sync_databases.clear()
    }

    /// Get the local database for this alpm instance.
    pub fn local_database<'a>(&'a self) -> Db<'a> {
        Db::new(&self.local_database, self)
    }

    /// Get a sync database with the given name for this alpm instance.
    pub fn sync_database<'a>(&'a self, name: impl AsRef<str>) -> Result<Db<'a>, Error> {
        let name = DbName::new(name)?;
        self.sync_databases.get(&name)
            .map(|db| Db::new(db, self))
            .ok_or(ErrorKind::DatabaseNotFound(name).into())
    }

    /// Get the parent database path
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    /// Get the parent database path
    pub fn database_extension(&self) -> &str {
        &self.database_extension
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
    /// Extension to use for names of sync databases.
    database_extension: Option<String>,
    /// todo
    gpg_path: Option<PathBuf>,
    /// The architecture to use when installing packages.
    arch: Option<String>,
}

impl Default for AlpmBuilder {
    fn default() -> Self {
        AlpmBuilder {
            root_path: None,
            database_path: None,
            database_extension: None,
            gpg_path: None,
            arch: None,
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

    /// Use custom database path
    pub fn with_database_extension(mut self, database_extension: impl AsRef<str>) -> Self {
        self.database_extension = Some(database_extension.as_ref().to_owned());
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
        util::check_valid_directory(&root_path)
            .context(ErrorKind::BadRootPath(root_path.clone()))?;

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
        util::check_valid_directory(&database_path)
            .context(ErrorKind::BadDatabasePath(database_path.clone()))?;

        let database_extension = self.database_extension.unwrap_or(
            DEFAULT_SYNC_DB_EXT.to_owned());
        if ! util::is_valid_db_extension(&database_extension) {
            return Err(ErrorKind::BadSyncDatabaseExt(database_extension).into());
        }
        debug!("database extension: .{}", &database_extension);

        let sync_db_path = database_path.join(SYNC_DB_DIR);
        debug!("sync database path: {}", sync_db_path.display());
        util::check_valid_directory(&sync_db_path)
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

        let arch = root_path.clone();
        debug!("gpg path: {}", gpg_path.display());

        // Get architecture of computer
        #[cfg(not(windows))]
        let arch = match self.arch {
            Some(arch) => arch,
            None => {
                let info = uname().context(ErrorKind::UnexpectedIo)?;
                info!("detected arch: {}", &info.machine);
                info.machine
            }
        };
        #[cfg(windows)]
        let arch = match self.arch {
            Some(arch) => arch,
            None => {
                error!("You must specify an arch on windows. I will assume x86_64.");
                "x86_64".into()
            }
        };
        debug!("arch: {}", &arch);

        signing::init(&gpg_path)?;

        let alpm = Alpm {
            local_database: DbBase::new_no_check_duplicates(
                DbName::LOCAL.clone(),
                SignatureLevel::Inherit,
                // we have to do the path because we don't have an Alpm yet.
                (DbName::LOCAL).path(&database_path, &database_extension),
            ),
            sync_databases: HashMap::new(),
            root_path,
            database_path,
            database_extension,
            lockfile,
            gpg_path,
            cache_dirs_paths: HashSet::new(),
            hook_dirs_paths: HashSet::new(),
            overwrite_file_paths: HashSet::new(),
            packages_no_upgrade: HashSet::new(),
            packages_no_extract: HashSet::new(),
            packages_ignore: HashSet::new(),
            groups_ignore: HashSet::new(),
            packages_assume_installed: HashSet::new(),
            arch,
            delta_ratio: 0.0,
            check_space: true,
            http_client: reqwest::Client::new(),
        };
        Ok(alpm)
    }
}
