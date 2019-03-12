//! Errors for serializing the alpm db format
use std::{error::Error as StdError, fmt, io, result::Result as StdResult};

use serde::ser;

/// Errors that can occur during serialization.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ErrorKind {
    /// Some i/o error occurred.
    Io,
    /// This format does not support the given operation
    Unsupported,
    /// A Serialize method returned a custom error.
    Custom,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            ErrorKind::Io => "an i/o error occured",
            ErrorKind::Unsupported => "tried to serialize an unsupported type/context",
            ErrorKind::Custom => "the type being serialized reported an error",
        })
    }
}

/// The error type for serialization
#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    inner: Option<Box<dyn StdError + Sync + Send + 'static>>,
}

impl Error {
    fn custom(inner: impl Into<Box<dyn StdError + Send + Sync + 'static>>) -> Self {
        Error {
            kind: ErrorKind::Custom,
            inner: Some(inner.into()),
        }
    }
    pub fn source(&self) -> Option<&(dyn StdError + Send + Sync + 'static)> {
        self.inner.as_ref().map(AsRef::as_ref)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error { kind, inner: None }
    }
}

impl StdError for Error {}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Error::custom(msg.to_string())
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error {
            kind: ErrorKind::Io,
            inner: Some(err.into()),
        }
    }
}

pub type Result<T> = StdResult<T, Error>;
