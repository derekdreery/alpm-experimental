//! Errors for serializing the alpm db format
use std::fmt::{self, Display};
use std::io;
use std::result::Result as StdResult;

use failure::{Context, Fail, format_err};
use serde::ser;

/// The error type for serialization
#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

/// Errors that can occur during serialization.
#[derive(Debug, Fail, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ErrorKind {
    /// Some i/o error occurred.
    #[fail(display = "an i/o error occured")]
    Io,
    /// This format does not support the given operation
    #[fail(display = "tried to serialize an unsupported type/context")]
    Unsupported,
    /// A Serialize method returned a custom error.
    #[fail(display = "the type being serialized reported an error")]
    Custom,
}

impl Error {
    /// Get the kind of this error
    pub fn kind(&self) -> ErrorKind {
        *self.inner.get_context()
    }

    /// Get a version of this error that implements `Fail`.
    ///
    /// Unfortunately we cannot implement `Fail` for this type because it conflicts with
    /// `std::error::Error`, which we must implement for serde.
    pub fn into_fail(self) -> Context<ErrorKind> {
        self.inner
    }
}

impl ::std::ops::Deref for Error {
    type Target = Context<ErrorKind>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &'static str {
        "unimplemented - use `Display` implementation"
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        None
    }
}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        format_err!("{}", msg).context(ErrorKind::Custom).into()
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        err.context(ErrorKind::Io).into()
    }
}

pub type Result<T> = StdResult<T, Error>;
