use failure::{Fail, Context, Backtrace};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>
}

#[derive(Debug, Clone, Fail, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ErrorKind {
    /// Indicates a general error creating lockfile, for example due to permissions.
    #[fail(display = "Cannot create the lockfile at {:?}", _0)]
    // this would be better displayed using Path::display, but can't do this in procedural macro.
    CannotAcquireLock(PathBuf),
    /// Indicates there was a lockfile already present.
    ///
    /// This can also happen if the library crashed, in which case it is safe to remove the file.
    #[fail(display = "Lockfile at {:?} already exists - you may delete it if you are certain no other instance is running", _0)]
    LockAlreadyExists(PathBuf),
    /// Indicates that a lock cannot be released
    #[fail(display = "Cannot release (remove) the lockfile at {:?}", _0)]
    CannotReleaseLock(PathBuf),
}

impl ErrorKind {
    /// Helper constructor for `CannotAcquireLock` variant
    pub fn cannot_acquire_lock(path: impl AsRef<Path>) -> ErrorKind {
        ErrorKind::CannotAcquireLock(path.as_ref().to_owned())
    }

    /// Helper constructor for `CannotReleaseLock` variant
    pub fn cannot_release_lock(path: impl AsRef<Path>) -> ErrorKind {
        ErrorKind::CannotReleaseLock(path.as_ref().to_owned())
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.inner.get_context()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error { inner: Context::new(kind) }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Self {
        Error { inner }
    }
}
