use std::path::{Path, PathBuf};
use std::ops::Deref;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::mem::ManuallyDrop;

use {Error, ErrorKind,};
use failure::{Fail, ResultExt};
use fs2::{FileExt, lock_contended_error};

/// A lockfile that cleans up after itself.
///
/// Inspired by `TempPath` in `tempfile` crate.
pub struct Lockfile {
    handle: ManuallyDrop<File>,
    path: PathBuf,
}

impl Lockfile {

    /// Create a lockfile at the given path
    ///
    /// # Panics
    ///
    /// Will panic if the path doesn't have a parent directory.
    pub fn create(path: impl AsRef<Path>) -> Result<Lockfile, Error> {
        let path = path.as_ref();

        // create parent directory if not exists (match libalpm behaviour)
        let dir = path.parent().expect("internal error: lockfile path must have a parent");
        fs::create_dir_all(dir).context(ErrorKind::cannot_acquire_lock(path))?;
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

        Ok(Lockfile {
            handle: ManuallyDrop::new(lockfile),
            path: path.to_owned()
        })

    }

    /// Get the path of the lockfile
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// Close the file.
    ///
    /// Use this instead of the destructor when you want to see if any errors occured when
    /// removing the file.
    pub fn close(self) -> Result<(), Error> {
        let Lockfile { handle, path } = self;
        let handle = ManuallyDrop::into_inner(handle);
        // close file
        drop(handle);

        // remove file
        fs::remove_file(path).context(ErrorKind::cannot_release_lock(path))?;
        debug!("Removed lockfile at {}", path.display());
        Ok(())
    }
}

impl Drop for Lockfile {
    fn drop(&mut self) {
        // we cannot return errors, but we can report them to logs
        if let Err(e) = self.handle.unlock() {
            error!("error releasing lockfile at {}: {}", self.path.display(), e);
        } else {
            debug!("lockfile unlocked at {}", self.path.display());
        }
        // Safe because we don't use handle after dropping it.
        unsafe {
            // close file
            ManuallyDrop::drop(&mut self.handle);
            // remove file
            if let Err(e) = fs::remove_file(&self.path) {
                warn!("could not remove lockfile at {}: {}", self.path.display(), e);
            }
            // path destructor will be run as usual.
            debug!("Removed lockfile at {}", self.path.display());
        }
    }
}