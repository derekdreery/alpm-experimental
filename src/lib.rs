extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate fs2;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate tempfile;


mod error;
mod util;

pub use error::{Error, ErrorKind};

use failure::{Fail, ResultExt};
use fs2::{lock_contended_error, FileExt};
use std::fs::{OpenOptions, File, create_dir_all, remove_file};
use std::mem::ManuallyDrop;
use std::io;
use std::path::{Path, PathBuf};

/// The name of the lockfile (hard-coded)
pub const LOCKFILE: &str = "db.lck";

/// Handle to an alpm instance. Uses a lockfile to prevent concurrent access to the
/// same db.
pub struct Alpm {
    /// Managed filesystem root (normally this will be "/")
    root_path: PathBuf,
    /// The path of the alpm package database
    database_path: PathBuf,
    /// The path of the lockfile, preventing multiple processes
    /// interacting with the database concurrently.
    lockfile_path: PathBuf,
    /// Path to the directory where gpg files are stored
    gpg_path: PathBuf,
    /// A handle on the lockfile. When this goes out of scope the file is unlocked.
    lockfile_handle: ManuallyDrop<File>,
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
}

/// Builder-pattern constructor for the Alpm struct.
///
/// Use `Alpm::new` to get an `AlpmBuilder`, use `AlpmBuilder::build` to get an `Alpm` instance.
///
/// See `Alpm` struct for field documentation.
pub struct AlpmBuilder {
    root_path: Option<PathBuf>,
    database_path: Option<PathBuf>,
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
    /// Use custom root path (defaults to "/")
    pub fn with_root_path(mut self, root_path: impl AsRef<Path>) -> Self {
        self.root_path = Some(root_path.as_ref().to_owned());
        self
    }

    /// Use custom database path (defaults to "$root/var/lib/pacman")
    pub fn with_database_path(mut self, database_path: impl AsRef<Path>) -> Self {
        self.database_path = Some(database_path.as_ref().to_owned());
        self
    }

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

        // todo sensible default database path on windows
        let database_path = match self.database_path {
            Some(path) => path,
            None => {
                let mut db_path = root_path.clone();
                db_path.push("var");
                db_path.push("lib");
                db_path.push("pacman");
                db_path
            }
        };
        debug!("database path: {}", database_path.display());

        // todo
        let gpg_path = root_path.clone();

        let lockfile_path = database_path.join(LOCKFILE);

        let lockfile_handle = create_lockfile(&lockfile_path)?;
        Ok(Alpm {
            root_path,
            database_path,
            lockfile_path,
            gpg_path,
            lockfile_handle,
            http_client: reqwest::Client::new(),
        })
    }
}

impl Drop for Alpm {
    fn drop(&mut self) {
        // discard error (will still be logged)
        remove_lockfile(self.lockfile_handle.clone(), &self.lockfile_path);
    }
}

/// Helper to create a lockfile and return the correct error on failure
///
/// # Panics
///
/// Will panic if the path doesn't have a parent directory.
fn create_lockfile(path: impl AsRef<Path>) -> Result<File, Error> {
    let path = path.as_ref();

    // create parent directory if not exists (match libalpm behaviour)
    let dir = path.parent().expect("internal error: lockfile path must have a parent");
    create_dir_all(dir).context(ErrorKind::cannot_acquire_lock(path))?;
    debug!("lockfile parent directories created/found at {}", dir.display());

    // create lockfile (or get a handle if file already exists)
    let mut lockfile_opts = OpenOptions::new();
    lockfile_opts.create(true)
        .read(true)
        .write(true);
    let lockfile = lockfile_opts.open(path)
        .context(ErrorKind::cannot_acquire_lock(path))?;
    debug!("lockfile created/found at {}", path.display());

    // lock lockfile
    match lockfile.try_lock_exclusive() {
        Ok(_) => (),
        Err(ref e) if e.kind() == lock_contended_error().kind() => {
            warn!("Lockfile at {} already present and locked, blocking until released",
                  path.display());
            lockfile.lock_exclusive().context(ErrorKind::cannot_acquire_lock(path))?;
        },
        Err(e) => Err(e.context(ErrorKind::cannot_acquire_lock(path)))?
    };
    debug!("lockfile locked at {}", path.display());

    Ok(lockfile)
}

/// Helper to remove a lockfile.
///
/// Returns an error if this was not possible
fn remove_lockfile(fd: File, path: impl AsRef<Path>) -> Result<(), Error> {
    let path = path.as_ref();

    // release lockfile
    fd.unlock().context(ErrorKind::cannot_release_lock(path))?;
    debug!("lockfile unlocked at {}", path.display());
    drop(fd);

    remove_file(path).context(ErrorKind::cannot_release_lock(path))?;
    debug!("lockfile removed at {}", path.display());
    Ok(())
}

