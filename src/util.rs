use std::cell::{RefCell, Ref};
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;

use failure::Fail;
use error::Error;

use reqwest::Url;

#[derive(Debug)]
pub struct NotADirectory;

impl fmt::Display for NotADirectory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", error::Error::description(self))
    }
}

impl error::Error for NotADirectory {
    fn description(&self) -> &str {
        "path exists and is not a directory"
    }
}

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
            warn!(
                "directory \"{}\" not found - attempting to create",
                path.display()
            );
            fs::create_dir_all(path)
        }
        Err(e) => Err(e),
    }
}

/// Check a string is a valid db extension.
///
/// For now, just allow ascii alphanumeric. This could be relaxed later.
pub fn is_valid_db_extension(ext: &str) -> bool {
    ext.chars().all(|ch| ch.is_alphanumeric())
}

pub struct DerefAsRef<D>(pub D);

impl<D: Deref> AsRef<D::Target> for DerefAsRef<D> {
    fn as_ref(&self) -> &D::Target {
        self.0.deref()
    }
}

pub struct DerefDerefAsRef<D>(pub D);

impl<D, D2> AsRef<D2::Target> for DerefDerefAsRef<D>
where
    D: Deref<Target=D2>,
    D2: Deref + 'static
{
    fn as_ref(&self) -> &D2::Target {
        let tmp = self.0.deref();
        tmp.deref()
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
            UrlOrStr::Str(ref s) => fmt::Display::fmt(s, f)
        }
    }
}

#[derive(Debug)]
pub struct Cache<T> {
    cache: RefCell<HashMap<String, Rc<T>>>,
    got_all: RefCell<bool>
}

impl<T> Cache<T> {
    pub fn new() -> Cache<T> {
        Cache {
            cache: RefCell::new(HashMap::new()),
            got_all: RefCell::new(false),
        }
    }

    pub fn get(&self, name: &str, getter: impl Fn(&str) -> Result<T, Error> + 'static)
        -> Result<Rc<T>, Error>
    {
        let cache = self.cache.borrow();
        if let Some(val) = cache.get(name) {
            return Ok(val.clone());
        }
        let val = Rc::new(getter(name)?);
        self.cache.borrow_mut().insert(name.to_owned(), val.clone());
        Ok(val)
    }

    pub fn get_all<F, F2, I>(&self,
                   get_list: F,
                   getter: F2)
        -> Result<Ref<HashMap<String, T>>, Error>
    where
        F: FnOnce() -> Result<I, Error> + 'static,
        F2: Fn(&str) -> Result<T, Error> + 'static,
        I: Iterator<Item=String>,
    {
        if self.got_all.borrow() {
            return Ok(self.cache.borrow())
        }

        let cache = self.cache.borrow_mut();
        for name in get_list() {
            if ! cache.contains(name) {
                let val = Rc::new(getter(name));
                cache.insert(name.to_owned(), val.clone());
            }
        }
        self.got_all.borrow_mut() = true;
        Ok(self.cache.borrow())
    }
}
