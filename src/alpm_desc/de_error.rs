//! Errors for serializing the alpm db format
use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::result::Result as StdResult;

use serde::de;

/// Errors that can occur during deserialization.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ErrorKind {
    /// This format does not support the given operation
    Unsupported(&'static str),
    /// The deserializer expected a bool
    ExpectedBool,
    /// The deserializer expected a hex-encoded byte
    ExpectedByte,
    /// The deserializer expected an unsigned integer
    ExpectedUnsigned,
    /// The deserializer expected a signed integer
    ExpectedSigned,
    /// The deserializer expected a float
    ExpectedFloat,
    /// The deserializer expected a char
    ExpectedChar,
    /// The deserializer expected a key (`%NAME%`)
    ExpectedKey,
    /// The deserializer expected an empty string
    ExpectedEmpty,
    /// A Serialize method returned a custom error.
    Custom(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind() {
            ErrorKind::Unsupported(msg) => write!(
                f,
                "tried to deserialize an unsupported type/context: {}",
                msg
            ),
            ErrorKind::ExpectedBool => write!(f, "expected a bool"),
            ErrorKind::ExpectedByte => write!(f, "expected a hex-encoded byte"),
            ErrorKind::ExpectedUnsigned => write!(f, "expected an unsigned integer"),
            ErrorKind::ExpectedSigned => write!(f, "expected a signed integer"),
            ErrorKind::ExpectedFloat => write!(f, "expected a float"),
            ErrorKind::ExpectedChar => write!(f, "expected a char"),
            ErrorKind::ExpectedKey => write!(f, "expected a key (e.g. `%NAME%`)"),
            ErrorKind::ExpectedEmpty => write!(f, "expected an empty string"),
            ErrorKind::Custom(msg) => {
                write!(f, "the type being deserialized reported an error: {}", msg)
            }
        }?;
        if let Some(cause) = &self.inner {
            write!(f, "\n{}", cause)?;
        }
        Ok(())
    }
}

/// The error type for deserialization
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    inner: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

impl Error {
    /// Get the kind of this error
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn sync_source(&self) -> Option<&(dyn StdError + Send + Sync + 'static)> {
        self.inner.as_ref().map(|b| b.as_ref())
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error { kind, inner: None }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner
            .as_ref()
            .map(|b| b.as_ref() as &(dyn StdError + 'static))
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        ErrorKind::Custom(format!("{}", msg)).into()
    }
}

pub type Result<T> = StdResult<T, Error>;
