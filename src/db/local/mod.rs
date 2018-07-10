use std::borrow::Cow;
use std::collections::hash_map::{self, HashMap};
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::iter::repeat;
use std::path::{Path, PathBuf};
use std::cell::{Ref, RefMut, RefCell};
use std::rc::{Rc, Weak};

use atoi::atoi;
use failure::{self, Fail, ResultExt, err_msg};

use alpm_desc::de;
use error::{ErrorKind, Error};
use db::{LOCAL_DB_NAME, SignatureLevel, DbStatus, DbUsage};
use Handle;

mod package;
pub use self::package::Package as LocalDbPackage;

const LOCAL_DB_VERSION_FILE: &str = "ALPM_DB_VERSION";
const LOCAL_DB_CURRENT_VERSION: u64 = 9;

/// A package database.
#[derive(Debug)]
pub struct LocalDatabase {
    inner: Rc<RefCell<LocalDatabaseInner>>
}

impl LocalDatabase {
    /// Helper to create a new database
    ///
    /// Path is the root path for databases.
    pub(crate) fn new(inner: Rc<RefCell<LocalDatabaseInner>>) -> LocalDatabase {
        LocalDatabase { inner }
    }

}

impl LocalDatabase {
    //type Pkg = Rc<LocalDbPackage>;
    //type PkgIter = RefMut<Values<String, Self::Pkg>>;
    //type Path = Ref<'static, PathBuf>;

    /// Get the name of this database
    pub fn name(&self) -> &str {
        LOCAL_DB_NAME
    }

    /// Get the path of the root file or directory for this database.
    pub fn path<'a>(&'a self) -> Ref<'a, Path> {
        Ref::map(self.inner.borrow(), |db| db.path.as_ref())
    }

    /// Get the status of this database.
    pub fn status(&self) -> Result<DbStatus, Error> {
        self.inner.borrow().status()
    }

    /// Get the number of packages.
    pub fn count(&self) -> usize {
        self.inner.borrow().package_count
    }

    /// Get a package in this database, if present.
    pub fn package(&self, name: impl AsRef<str>, version: impl AsRef<str>)
        -> Result<Rc<LocalDbPackage>, Error>
    {
        self.inner.borrow().package(name, version)
    }

    /// Iterate over all packages.
    ///
    /// The closure allows propagating errors, but errors can occur outside of the closure of type
    /// `Error`, which is why the `From` bound exists. If your closure can't error, just use
    /// `E = Error`.
    ///
    /// Because the closure receives reference counted packages, they are cheap to clone, and can
    /// be collected into a Vec if that is desired.
    pub fn packages<E, F>(&self, f: F) -> Result<(), E>
    where F: FnMut(Rc<LocalDbPackage>) -> Result<(), E>,
          E: From<Error>
    {
        self.inner.borrow().packages(f)
    }

    /// Get the latest version of a package in this database, if a version is present.
    pub fn package_latest(&self, name: impl AsRef<str>) -> Result<Rc<LocalDbPackage>, Error> {
        self.inner.borrow().package_latest(name)
    }
}

/// A package database.
#[derive(Debug)]
pub struct LocalDatabaseInner {
    handle: Weak<RefCell<Handle>>,
    /// The level of signature verification required to accept packages
    sig_level: SignatureLevel,
    /// Which operations this database will be used for.
    usage: DbUsage,
    /// The database path.
    path: PathBuf,
    /// The package cache (HashMap of package name to package version to package, which lazily
    /// gets info from disk)
    package_cache: HashMap<String, HashMap<String, RefCell<MaybePackage>>>,
    /// Count of the number of packages (cached)
    package_count: usize,
}

impl LocalDatabaseInner {

    /// Helper to create a new database
    ///
    /// Path is the root path for databases.
    ///
    /// The database folder will be read to get a cache of package names.
    // This function must not panic, it is UB to panic here.
    pub(crate) fn new(handle: &Rc<RefCell<Handle>>, sig_level: SignatureLevel)
        -> LocalDatabaseInner
    {
        //  path is `$db_path SEP $local_db_name` for local
        let path = handle.borrow().database_path.join(LOCAL_DB_NAME);
        LocalDatabaseInner {
            handle: Rc::downgrade(handle),
            sig_level,
            usage: DbUsage::default(),
            path,
            package_cache: HashMap::new(),
            package_count: 0,
        }
    }

    /// Helper to create a new version file for the local database.
    #[inline]
    fn create_version_file(&self) -> io::Result<()> {
        let mut version_file = fs::File::create(&self.path)?;
        // Format is number followed by single newline
        writeln!(version_file, "{}", LOCAL_DB_CURRENT_VERSION)?;
        Ok(())
    }

    /// Get a package from the database
    fn package(&self, name: impl AsRef<str>, version: impl AsRef<str>)
        -> Result<Rc<LocalDbPackage>, Error>
    {
        let name = name.as_ref();
        let version = version.as_ref();

        self.package_cache
            .get(name)
            .and_then(|versions| versions.get(version))
            .ok_or(ErrorKind::InvalidLocalPackage(name.to_owned()))?
            .borrow_mut()
            .load(self.handle.clone())
    }

    /// Get the latest version of a package from the database.
    ///
    /// There should only be one version of a package installed at any time,
    /// so this function is kinda useless.
    fn package_latest(&self, name: impl AsRef<str>) -> Result<Rc<LocalDbPackage>, Error> {
        let name = name.as_ref();

        self.package_cache
            .get(name)
            .and_then(|versions| {
                let mut versions_iter = versions.keys();
                let mut version = versions_iter.next().unwrap();
                for v in versions_iter {
                    if v > version {
                        version = v;
                    }
                }
                versions.get(version)
            })
            .ok_or(ErrorKind::InvalidLocalPackage(name.to_owned()))?
            .borrow_mut()
            .load(self.handle.clone())
    }

    fn packages<'a, E, F>(&'a self, mut f: F) -> Result<(), E>
        where F: FnMut(Rc<LocalDbPackage>) -> Result<(), E>,
              E: From<Error>
    {
        for pkg in self.package_cache
            .values()
            .flat_map(|versions| versions.values())
            .map(|pkg| pkg.borrow_mut().load(self.handle.clone()))
        {
            f(pkg?)?;
        }
        Ok(())
    }

    /// Get the status of this database.
    ///
    /// This does not validate installed packages, just the internal structure of the database.
    fn status(&self) -> Result<DbStatus, Error> {
        let md = match fs::metadata(&self.path) {
            Ok(md) => md,
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(DbStatus::Missing);
            }
            Err(e) => {
                return Err(e.context(ErrorKind::UnexpectedIo).into())
            }
        };

        if ! md.is_dir() {
            return Ok(DbStatus::Exists { valid: false });
        }

        debug!("checking local database version");
        let valid = match fs::read(self.path.join(&LOCAL_DB_VERSION_FILE)) {
            Ok(version_raw) => {
                // Check version is up to date.
                if let Some(version) = atoi::<u64>(&version_raw) {
                    if version == LOCAL_DB_CURRENT_VERSION {
                        true
                    } else {
                        warn!(r#"local database version is "{}" which is not the latest ("{}")"#,
                              version, LOCAL_DB_CURRENT_VERSION);
                        false
                    }
                } else {
                    error!(r#""{}" is not a valid version"#,
                           String::from_utf8_lossy(&version_raw));
                    false
                }
            },
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                // check directory is empty and create version file
                debug!("local database version file not found - creating");
                match fs::read_dir(&self.path) {
                    Ok(ref mut d) => match d.next() {
                        Some(_) => false,
                        None => match self.create_version_file() {
                            Ok(_) => true,
                            Err(e) => {
                                error!("could not create version file for local database at {}",
                                       self.path.display());
                                error!("caused by {}", e);
                                false
                            }
                        }
                    }
                    Err(e) => {
                        error!("could not check contents of local database directory at {}",
                               self.path.display());
                        error!("caused by {}", e);
                        false
                    }
                }
            },
            Err(e) => {
                error!("could not read version file for the local database at {}",
                       self.path.display());
                error!("caused by {}", e);
                false
            }
        };
        Ok(DbStatus::Exists { valid })
    }


    /// Load all package names into the cache, and validate the database
    // The syscalls for this function are a single readdir and a stat per subentry
    pub(crate) fn populate_package_cache(&mut self) -> Result<(), Error> {
        let mut count = 0;
        debug!(r#"searching for local packages in "{}""#, self.path.display());
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            if ! entry.metadata()?.is_dir() {
                // Check ALPM_DB_VERSION
                if entry.file_name() == OsStr::new(LOCAL_DB_VERSION_FILE) {
                } else {
                    // ignore extra files for now (should probably error)
                    warn!("Unexpected file {} found in local db directory",
                          entry.path().display());
                }
                continue;
            }
            let path = entry.path();
            // Non-utf8 is hard until https://github.com/rust-lang/rfcs/pull/2295 lands
            let file_name = entry.file_name()
                .into_string()
                .expect("non-utf8 package names not yet supported");
            let (name, version) = split_package_dirname(&file_name)
                .ok_or(ErrorKind::InvalidLocalPackage(file_name.to_owned()))?;
            debug!(r#"found "{}", version: "{}""#, name, version);
            let mut versions_map = self.package_cache.entry(name.to_owned())
                .or_insert(HashMap::new());
            let new_pkg = MaybePackage::new(path, name, version);
            if versions_map.insert(version.to_owned(), RefCell::new(new_pkg)).is_some() {
                // This should not be possible (since name comes from unique filename)
                panic!("Found package in localdb with duplicate name/version");
            }
            count += 1;
        }
        self.package_count = count;
        Ok(())
    }
}

/// A lazy-loading package
#[derive(Debug, Clone, PartialEq)]
enum MaybePackage {
    /// Not loaded the package yet
    Unloaded {
        path: PathBuf,
        name: String,
        version: String
    },
    /// Loaded the package
    Loaded(Rc<LocalDbPackage>)
}

impl MaybePackage {
    /// Create an unloaded package
    fn new(path: impl Into<PathBuf>,
           name: impl Into<String>,
           version: impl Into<String>) -> MaybePackage
    {
        MaybePackage::Unloaded {
            path: path.into(),
            name: name.into(),
            version: version.into()
        }
    }

    /// Load the package if necessary and return it
    fn load(&mut self, handle: Weak<RefCell<Handle>>) -> Result<Rc<LocalDbPackage>, Error> {
        match self {
            MaybePackage::Unloaded { path, name, version } => {
                // todo find a way to avoid cloning `path`
                let pkg = Rc::new(LocalDbPackage::from_local(path.clone(), name, version, handle)?);
                *self = MaybePackage::Loaded(pkg.clone());
                Ok(pkg)
            },
            MaybePackage::Loaded(pkg) => Ok(pkg.clone())
        }
    }
}

/// Keys for hashtable of packages.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct PackageKey<'a> {
    /// The package name.
    name: Cow<'a, str>,
    /// The package version.
    version: Cow<'a, str>,
}

impl<'a> PackageKey<'a> {
    /// Create a PackageKey from references
    #[inline]
    fn from_borrowed(name: &'a str, version: &'a str) -> PackageKey<'a> {
        PackageKey {
            name: Cow::Borrowed(name.as_ref()),
            version: Cow::Borrowed(version.as_ref()),
        }
    }

    /// Create a PackageKey from owned values
    #[inline]
    fn from_owned(name: impl Into<String>, version: impl Into<String>) -> PackageKey<'static> {
        PackageKey {
            name: Cow::Owned(name.into()),
            version: Cow::Owned(version.into()),
        }
    }
}

/// If the name has at least 2 hyphens ('-'), split at the second from last
fn split_package_dirname(input: &str) -> Option<(&str, &str)> {
    let idx = input.rmatch_indices('-').skip(1).next()?.0;
    let start2 = idx + '-'.len_utf8();
    Some((&input[0..idx], &input[start2..]))
}

#[test]
fn test_split_package_dirname() {
    assert_eq!(split_package_dirname("abc-1223123-34"), Some(("abc", "1223123-34")));
}
