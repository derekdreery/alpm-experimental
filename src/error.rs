use failure::{Backtrace, Context, Fail};
use std::fmt;
use std::path::{Path, PathBuf};

use db::DbName;

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Debug, Clone, Fail, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ErrorKind {
    /// Indicates that the specified root directory is not valid, either because it is
    /// inaccessible, or because it is not a directory.
    #[fail(display = "The root path \"{:?}\" does not point to a valid directory on the system.",
           _0)]
    // this would be better displayed using Path::display, but can't do this in procedural macro.
    BadRootPath(PathBuf),
    /// Indicates that the specified database directory is not valid, either because it is
    /// inaccessible, or because it is not a directory.
    #[fail(display = "The database path \"{:?}\" does not point to a valid directory on the system.",
           _0)]
    BadDatabasePath(PathBuf),
    /// Indicates that the specified sync database directory is not valid, either because it is
    /// inaccessible, or because it is not a directory.
    #[fail(display = "The sync database path \"{:?}\" does not point to a valid directory on the system.",
           _0)]
    BadSyncDatabasePath(PathBuf),
    /// Indicates a general error creating lockfile, for example due to permissions.
    #[fail(display = "Cannot create the lockfile at \"{:?}\"", _0)]
    CannotAcquireLock(PathBuf),
    /// Indicates there was a lockfile already present.
    ///
    /// This can also happen if the library crashed, in which case it is safe to remove the file.
    #[fail(display = "Lockfile at \"{:?}\" already exists - you may delete it if you are certain no other instance is running",
           _0)]
    LockAlreadyExists(PathBuf),
    /// Indicates that a lock cannot be released
    #[fail(display = "Cannot release (remove) the lockfile at \"{:?}\"", _0)]
    CannotReleaseLock(PathBuf),
    /// A given database name is invalid.
    #[fail(display = "Cannot use \"{}\" as a database name - it is not a valid directory name",
           _0)]
    InvalidDatabaseName(String),
    /// A given database name already exists.
    #[fail(display = "Database with name \"{}\" already exists", _0)]
    DatabaseAlreadyExists(DbName),
    /// There was an unexpected error when creating a database.
    #[fail(display = "Could not create database \"{}\" on the filesystem.", _0)]
    CannotCreateDatabase(DbName),
}

impl ErrorKind {}

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
        Error {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Self {
        Error { inner }
    }
}
