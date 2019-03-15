//! A library to manipulate a system managed by the Alpm (Arch Linux Package Manager).
//!
//!TODO use signal_hook to handle interrupt etc. and avoid leaving the computer in an unusable
//!state.
#[cfg(not(unix))]
compile_error!("Only works on unix for now");

mod error;
//mod signing;
mod util;
pub mod version;

pub mod alpm_desc;
pub mod db;
pub mod mutation;
mod package;

use crate::db::{
    LocalDatabase, LocalDatabaseInner, SignatureLevel, SyncDatabase, SyncDatabaseInner, SyncDbName,
    DEFAULT_SYNC_DB_EXT, SYNC_DB_DIR,
};

use lockfile::Lockfile;
use uname::uname;

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    io,
    path::PathBuf,
    rc::Rc,
};

pub use crate::{
    error::{Error, ErrorContext, ErrorKind},
    package::Package,
};

/// The name of the lockfile (hard-coded).
const LOCKFILE: &str = "db.lck";

/// The main alpm object that owns the system handle.
pub struct Alpm {
    handle: Rc<RefCell<Handle>>,
}

impl Alpm {
    /// Create a builder for a new alpm instance.
    ///
    /// # Examples
    ///
    /// Create a new instance using the defaults
    /// ```no_run
    /// # use alpm::Alpm;
    /// let alpm = Alpm::new().build();
    /// ```
    ///
    /// Create a new instance for a chroot environment
    /// ```no_run
    /// # use alpm::Alpm;
    /// let alpm = Alpm::new()
    ///     .with_root_path("/my/chroot")
    ///     .build();
    /// ```
    pub fn new() -> AlpmBuilder {
        Default::default()
    }

    /// Get the local database for this alpm instance.
    pub fn local_database(&self) -> LocalDatabase {
        LocalDatabase::new(match &self.handle.borrow().local_database {
            Some(db) => db.clone(),
            // The local database is always Some before this can be called.
            None => unreachable!(),
        })
    }

    /// Get a sync database with the given name for this alpm instance.
    ///
    /// The database is only valid while the `Alpm` instance is in scope. Once it is dropped, all
    /// calls to the database will error.
    pub fn sync_database(&self, name: impl AsRef<str>) -> Result<SyncDatabase, Error> {
        let name = name.as_ref();
        let db_name = SyncDbName::new(name)?;
        let db = self
            .handle
            .borrow()
            .sync_databases
            .get(&db_name)
            .map(Clone::clone);
        // Second stage to release borrow
        let db = match db {
            Some(db) => db,
            None => self.register_sync_database(&db_name),
        };

        let name = db_name.into();
        Ok(SyncDatabase::new(db, name))
    }

    pub fn sync_databases<F>(&self, mut f: F)
    where
        F: FnMut(SyncDatabase),
    {
        for (name, db) in self.handle.borrow().sync_databases.iter() {
            f(SyncDatabase::new(db.clone(), name.to_string()));
        }
    }

    /// Register a new sync database
    ///
    /// The name must not match `LOCAL_DB_NAME`.
    fn register_sync_database(&self, name: &SyncDbName) -> Rc<RefCell<SyncDatabaseInner>> {
        let handle = self.handle.clone();
        let new_db = SyncDatabaseInner::new(handle, name.clone(), SignatureLevel::default());
        let new_db = Rc::new(RefCell::new(new_db));
        if self
            .handle
            .borrow_mut()
            .sync_databases
            .insert(name.clone(), new_db.clone())
            .is_some()
        {
            panic!(r#"internal error: database "{}" already registered"#, name);
        };
        new_db
    }

    /// Are there any databases already registered with the given name
    pub fn sync_database_exists(&self, name: impl AsRef<str>) -> bool {
        match SyncDbName::new(name.as_ref()) {
            Ok(name) => self.handle.borrow().sync_databases.contains_key(&name),
            Err(_) => false,
        }
    }

    /// Unregister a sync database.
    ///
    /// Database is left on the filesystem and will not be touched after this is called.
    pub fn unregister_sync_database(&mut self, name: impl AsRef<str>) {
        let name = name.as_ref();
        let name = match SyncDbName::new(name) {
            Ok(name) => name,
            Err(_) => {
                log::warn!(
                    "could not unregister a database with name \"{}\" (name not valid)",
                    name
                );
                return;
            }
        };
        if !self
            .handle
            .borrow_mut()
            .sync_databases
            .remove(&name)
            .is_none()
        {
            log::warn!(
                "could not unregister a database with name \"{}\" (not found)",
                name
            );
        }
    }

    /// Helper function to deregister all sync databases from the alpm instance.
    ///
    /// The databases will continue to exist while there are handles to them
    /// (from `sync_database`).
    pub fn unregister_all_sync_databases(&mut self) {
        self.handle.borrow_mut().sync_databases.clear()
    }

    // The following could avoid cloning, but the types are complex and it is unlikely to be a
    // performance bottleneck

    /// Get the parent database path
    pub fn database_path(&self) -> PathBuf {
        self.handle.borrow().database_path.clone()
    }

    /// Get the parent database path
    pub fn database_extension(&self) -> String {
        self.handle.borrow().database_extension.clone()
    }

    /// Get the root of this alpm instance.
    pub fn root_path(&self) -> PathBuf {
        self.handle.borrow().root_path.clone()
    }
}

/// Handle to an alpm instance. Uses a lockfile to prevent concurrent processes accessing the
/// same db.
#[derive(Debug)]
struct Handle {
    /// The local package database
    local_database: Option<Rc<RefCell<LocalDatabaseInner>>>,
    /// A list of all sync databases
    ///
    /// We can access these concurrently, as they manage their own mutability.
    sync_databases: HashMap<SyncDbName, Rc<RefCell<SyncDatabaseInner>>>,
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
    cache_directories: Vec<PathBuf>,
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

impl Handle {
    /// Are there any databases already registered with the given name
    fn sync_database_registered(&self, name: &SyncDbName) -> bool {
        self.sync_databases.contains_key(&name)
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
    /// A set of locations that we can download packages to.
    cache_directories: Vec<PathBuf>,
    /// A set of packages to skip during upgrade.
    packages_no_upgrade: HashSet<String>,
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
            cache_directories: Vec::new(),
            packages_no_upgrade: HashSet::new(),
            arch: None,
        }
    }
}

impl AlpmBuilder {
    /// Use custom root path.
    pub fn with_root_path(mut self, root_path: impl Into<PathBuf>) -> Self {
        self.root_path = Some(root_path.into());
        self
    }

    /// Use custom database path
    pub fn with_database_path(mut self, database_path: impl Into<PathBuf>) -> Self {
        self.database_path = Some(database_path.into());
        self
    }

    /// Use custom database path
    pub fn with_database_extension(mut self, database_extension: impl Into<String>) -> Self {
        self.database_extension = Some(database_extension.into());
        self
    }
    /// Use custom gpg location
    pub fn with_gpg_path(mut self, gpg_path: impl Into<PathBuf>) -> Self {
        self.gpg_path = Some(gpg_path.into());
        self
    }

    /// Add a cache directory
    pub fn with_cache_directory(mut self, cache_directory: impl Into<PathBuf>) -> Self {
        self.cache_directories.push(cache_directory.into());
        self
    }

    /// Mark a package as no-upgrade.
    pub fn mark_no_upgrade(mut self, no_upgrade: impl Into<String>) -> Self {
        self.packages_no_upgrade.insert(no_upgrade.into());
        self
    }

    /// Build the alpm instance.
    pub fn build(mut self) -> Result<Alpm, Error> {
        // todo check that root path is not relative.
        #[cfg(windows)]
        let root_path = self.root_path.unwrap_or("C:\\".into());
        #[cfg(not(windows))]
        let root_path = self.root_path.unwrap_or("/".into());
        log::debug!("root path: {}", root_path.display());
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

        log::debug!("database path: {}", database_path.display());
        // todo should I be checking database_path is valid here?

        let database_extension = self
            .database_extension
            .unwrap_or(DEFAULT_SYNC_DB_EXT.to_owned());
        if !is_valid_db_extension(&database_extension) {
            return Err(ErrorKind::BadSyncDatabaseExt(database_extension).into());
        }
        log::debug!("database extension: .{}", &database_extension);

        let sync_db_path = database_path.join(SYNC_DB_DIR);
        log::debug!("sync database path: {}", sync_db_path.display());
        util::check_valid_directory(&sync_db_path)
            .context(ErrorKind::BadSyncDatabasePath(sync_db_path.clone()))?;

        let lockfile_path = database_path.join(LOCKFILE);
        log::debug!("lockfile path: {}", lockfile_path.display());

        let lockfile = Lockfile::create(&lockfile_path).map_err(|e| {
            let kind = e.kind();
            if kind == io::ErrorKind::AlreadyExists {
                Error::lock_already_exists(lockfile_path, e)
            } else {
                Error::cannot_acquire_lock(lockfile_path, e)
            }
        })?;

        // todo
        let gpg_path = root_path.clone();
        log::debug!("gpg path: {}", gpg_path.display());

        self.cache_directories.dedup();
        if self.cache_directories.is_empty() {
            self.cache_directories.push("/var/cache/pacman/pkg".into());
        }

        // Get architecture of computer
        #[cfg(not(windows))]
        let arch = match self.arch {
            Some(arch) => arch,
            None => {
                let info = uname().context(ErrorKind::UnexpectedIo)?;
                log::info!("detected arch: {}", &info.machine);
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
        log::debug!("arch: {}", &arch);

        //signing::init(&gpg_path)?;

        // Chicken-and-egg problem for local_database
        let handle = Rc::new(RefCell::new(Handle {
            local_database: None,
            sync_databases: HashMap::new(),
            root_path,
            database_path,
            database_extension,
            lockfile,
            gpg_path,
            cache_directories: self.cache_directories,
            hook_dirs_paths: HashSet::new(),
            overwrite_file_paths: HashSet::new(),
            packages_no_upgrade: self.packages_no_upgrade,
            packages_no_extract: HashSet::new(),
            packages_ignore: HashSet::new(),
            groups_ignore: HashSet::new(),
            packages_assume_installed: HashSet::new(),
            arch,
            delta_ratio: 0.0,
            check_space: true,
            http_client: reqwest::Client::new(),
        }));
        let mut local_database = LocalDatabaseInner::new(&handle, SignatureLevel::default());
        local_database.populate_package_cache()?;
        handle.borrow_mut().local_database = Some(Rc::new(RefCell::new(local_database)));
        Ok(Alpm { handle })
    }
}

/// Check a string is a valid db extension.
///
/// For now, just allow ascii alphanumeric. This could be relaxed later.
fn is_valid_db_extension(ext: &str) -> bool {
    ext.chars().all(|ch| ch.is_alphanumeric())
}
