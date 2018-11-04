use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use failure::Fail;

use reqwest::Url;

#[derive(Debug, Copy, Clone)]
pub struct NotADirectory;

impl fmt::Display for NotADirectory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("path exists and is not a directory")
    }
}

impl error::Error for NotADirectory {}

/// Checks a path is a valid accessible directory.
///
/// If the directory is missing, attempt to create it. All other errors are returned.
pub fn check_valid_directory(path: &Path) -> io::Result<()> {
    match fs::metadata(path) {
        Ok(attr) => {
            if attr.is_dir() {
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::AlreadyExists, NotADirectory))
            }
        }
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
            // try to create and return any error
            log::warn!(
                "directory \"{}\" not found - attempting to create",
                path.display()
            );
            fs::create_dir_all(path)
        }
        Err(e) => Err(e),
    }
}

/// This structure only exists until `impl TryFrom<AsRef<str>> for Url` exists.
pub enum UrlOrStr {
    /// A url
    Url(Url),
    /// A borrowed string
    Str(String),
}

impl UrlOrStr {
    pub fn into_url(self) -> Result<Url, (String, impl Fail)> {
        match self {
            UrlOrStr::Url(url) => Ok(url),
            UrlOrStr::Str(s) => s.parse().map_err(|e| (s, e)),
        }
    }
}

impl From<Url> for UrlOrStr {
    fn from(url: Url) -> UrlOrStr {
        UrlOrStr::Url(url)
    }
}

impl<'a> From<&'a str> for UrlOrStr {
    fn from(s: &str) -> UrlOrStr {
        UrlOrStr::Str(s.to_owned())
    }
}

impl From<String> for UrlOrStr {
    fn from(s: String) -> UrlOrStr {
        UrlOrStr::Str(s)
    }
}

impl fmt::Display for UrlOrStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UrlOrStr::Url(ref url) => fmt::Display::fmt(url, f),
            UrlOrStr::Str(ref s) => fmt::Display::fmt(s, f),
        }
    }
}
