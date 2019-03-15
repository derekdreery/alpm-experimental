use mtree;
use std::{error::Error as StdError, fmt, io, path::PathBuf};

/// The different kinds of error that can occur in this library.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ErrorKind {
    /// Indicates that the specified root directory is not valid, either because it is
    /// inaccessible, or because it is not a directory.
    BadRootPath(PathBuf),
    /// Indicates that the specified database directory is not valid, either because it is
    /// inaccessible, or because it is not a directory.
    BadDatabasePath(PathBuf),
    /// The extension provided is not a valid database extension.
    BadSyncDatabaseExt(String),
    /// Indicates that the specified sync database directory is not valid, either because it is
    /// inaccessible, or because it is not a directory.
    BadSyncDatabasePath(PathBuf),
    /// Indicates a general error creating lockfile, for example due to permissions.
    CannotAcquireLock(PathBuf),
    /// Indicates there was a lockfile already present.
    ///
    /// This can also happen if the library crashed, in which case it is safe to remove the file.
    LockAlreadyExists(PathBuf),
    /// Indicates that a lock cannot be released
    CannotReleaseLock(PathBuf),
    /// A given database name is invalid.
    InvalidDatabaseName(String),
    /// A given database name already exists.
    DatabaseAlreadyExists(String),
    /// Cannot find a database with the given name.
    DatabaseNotFound(String),
    /// There was an unexpected error when creating a database.
    CannotCreateDatabase(String),
    /// Could not query database on the filesystem.
    CannotQueryDatabase(String),
    /// Failed to add server with given url to database.
    CannotAddServerToDatabase {
        url: String,
        database: String,
    },
    InvalidLocalPackage(String),
    InvalidSyncPackage(String),
    /// There was an error when getting/updating the database version.
    DatabaseVersion(String),
    /// Error configuring gpg.
    Gpgme,
    /// A signature was missing.
    SignatureMissing,
    /// A signature did not match.
    SignatureIncorrect,
    /// An unexpected error occurred during signature verification.
    UnexpectedSignature(String),
    /// The main handle has been dropped
    UseAfterDrop,
    /// There was an unexpected i/o error
    UnexpectedIo,
    /// There was an unexpected mtree parsing error
    UnexpectedMtree,
    /// There was an unexpected reqwest error
    UnexpectedReqwest,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::BadRootPath(path) => write!(f, "The root path \"{}\" does not point to a valid directory on the system.", path.display()),
            ErrorKind::BadDatabasePath(path) => write!(f, "The database path \"{}\" does not point to a valid directory on the system.", path.display()),
            ErrorKind::BadSyncDatabaseExt(ext) => write!(f, "\"{}\" is not a valid database extension.", ext),
            ErrorKind::BadSyncDatabasePath(path) => write!(f, "The sync database path \"{}\" does not point to a valid directory on the system.", path.display()),
            ErrorKind::CannotAcquireLock(path) => write!(f, "Cannot create the lockfile at \"{}\"", path.display()),
            ErrorKind::LockAlreadyExists(path) => write!(f, "Lockfile at \"{}\" already exists - you may delete it if you are certain no other instance is running", path.display()),
            ErrorKind::CannotReleaseLock(path) => write!(f, "Cannot release (remove) the lockfile at \"{}\"", path.display()),
            ErrorKind::InvalidDatabaseName(name) => write!(f, "Cannot use \"{}\" as a database name - it is not a valid directory name", name),
            ErrorKind::DatabaseAlreadyExists(name) => write!(f, "Database with name \"{}\" already exists", name),
            ErrorKind::DatabaseNotFound(name) => write!(f, "Cannot find database with name \"{}\"", name),
            ErrorKind::CannotCreateDatabase(name) => write!(f, "Could not create database \"{}\" on the filesystem.", name),
            ErrorKind::CannotQueryDatabase(name) => write!(f, "Could not query database \"{}\" on the filesystem.", name),
            ErrorKind::CannotAddServerToDatabase { url, database } => write!(f, "Cannot add server with url \"{}\" to database \"{}\".", url, database),
            ErrorKind::InvalidLocalPackage(name) => write!(f, "A package (\"{}\") in the local database was invalid", name),
            ErrorKind::InvalidSyncPackage(name) => write!(f, "A package (\"{}\") in a sync database was invalid", name),
            ErrorKind::DatabaseVersion(name) => write!(f, "there was an unexpected error getting/updating the version for database \"{}\"", name),
            ErrorKind::Gpgme => write!(f, "there was an error configuring gpgme"),
            ErrorKind::SignatureMissing => write!(f, "a signature was missing"),
            ErrorKind::SignatureIncorrect => write!(f, "a signature did not match"),
            ErrorKind::UnexpectedSignature(name) => write!(f, "an unexpected error occurred while processing a signature for \"{}\"", name),
            ErrorKind::UseAfterDrop => write!(f, "no operations are possible after the main handle has been dropped"),
            ErrorKind::UnexpectedIo => write!(f, "there was an unexpected i/o error"),
            ErrorKind::UnexpectedMtree => write!(f, "there was an unexpected mtree parsing error"),
            ErrorKind::UnexpectedReqwest => write!(f, "there was an unexpected reqwest error"),
        }
    }
}

/// The main error type for this library.
#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    inner: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

impl Error {
    #[inline]
    // some constructors
    fn from_parts(
        kind: ErrorKind,
        inner: Option<impl Into<Box<dyn StdError + Send + Sync + 'static>>>,
    ) -> Self {
        Error {
            kind,
            inner: inner.map(Into::into),
        }
    }
    pub fn lock_already_exists(path: impl Into<PathBuf>, err: io::Error) -> Self {
        Self::from_parts(ErrorKind::LockAlreadyExists(path.into()), Some(err))
    }
    pub fn cannot_acquire_lock(path: impl Into<PathBuf>, err: io::Error) -> Self {
        Self::from_parts(ErrorKind::CannotAcquireLock(path.into()), Some(err))
    }
    pub fn invalid_local_package(
        name: impl Into<String>,
        err: impl Into<Box<dyn StdError + Send + Sync + 'static>>,
    ) -> Self {
        Self::from_parts(ErrorKind::InvalidLocalPackage(name.into()), Some(err))
    }
    pub fn invalid_sync_package(
        name: impl Into<String>,
        err: impl Into<Box<dyn StdError + Send + Sync + 'static>>,
    ) -> Self {
        Self::from_parts(ErrorKind::InvalidSyncPackage(name.into()), Some(err))
    }

    /// Add in a source
    pub fn with_source(
        mut self,
        inner: impl Into<Box<dyn StdError + Send + Sync + 'static>>,
    ) -> Self {
        self.inner = Some(inner.into());
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner
            .as_ref()
            .map(|i| &**i as &(dyn StdError + 'static))
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error { kind, inner: None }
    }
}

impl From<io::Error> for Error {
    fn from(cause: io::Error) -> Self {
        Error::from_parts(ErrorKind::UnexpectedIo, Some(cause))
    }
}

impl From<mtree::Error> for Error {
    fn from(from: mtree::Error) -> Error {
        match from {
            mtree::Error::Io(e) => Error::from(e),
            mtree::Error::Parser(e) => Error::from_parts(ErrorKind::UnexpectedMtree, Some(e)),
        }
    }
}

/// Helper trait to help working with `Result<T, Error>` where `Error` is our error.
pub trait ErrorContext<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    /// Takes any result and makes wraps the error in the given context.
    fn context(self, context: ErrorKind) -> Result<T, Error>;
    /// Takes any result and makes wraps the error in the context given by the function.
    fn with_context<F>(self, f: F) -> Result<T, Error>
    where
        F: FnOnce(&E) -> ErrorKind;
}

impl<T, E> ErrorContext<T, E> for Result<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn context(self, context: ErrorKind) -> Result<T, Error> {
        self.map_err(|err| Error {
            kind: context,
            inner: Some(Box::new(err)),
        })
    }

    fn with_context<F>(self, f: F) -> Result<T, Error>
    where
        F: FnOnce(&E) -> ErrorKind,
    {
        self.map_err(|err| {
            let kind = f(&err);
            Error {
                kind,
                inner: Some(Box::new(err)),
            }
        })
    }
}
