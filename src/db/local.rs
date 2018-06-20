use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::cell::RefCell;
use std::rc::Rc;

use atoi::atoi;
use failure::{self, Fail, ResultExt, err_msg};

use alpm_desc::de;
use error::{ErrorKind, Error};
use package::Package;
use db::{LOCAL_DB_NAME, Database, SignatureLevel, DbStatus, DbUsage};
use util::Cache;

const LOCAL_DB_VERSION_FILE: &str = "ALPM_DB_VERSION";
const LOCAL_DB_CURRENT_VERSION: u64 = 9;

/// A package database.
pub struct LocalDatabase {
    inner: RefCell<LocalDatabaseInner>
}

impl LocalDatabase {
    /// Helper to create a new database
    ///
    /// Path is the root path for databases.
    pub(crate) fn new(mut path: PathBuf, sig_level: SignatureLevel) -> LocalDatabase
    {
        LocalDatabase {
            inner: RefCell::new(LocalDatabaseInner::new(path, sig_level))
        }
    }

}

impl Database for LocalDatabase {
    type PkgIter = ();

    /// Get the name of this database
    fn name(&self) -> &str {
        LOCAL_DB_NAME
    }

    /// Get the path of the root file or directory for this database.
    fn path(&self) -> Ref<Path> {
        Ref::map(self.inner.borrow(), |db| db.path)
    }

    /// Get the status of this database.
    fn status(&self) -> Result<DbStatus, Error> {
        self.inner.borrow().status()
    }

    /// Get the packages in this database
    fn package(&self, name: &str) -> Result<Rc<Package>, Error> {
        self.inner.borrow_mut().package(name)
    }

    fn packages(&self) -> Result<impl Iterator<Item=Package>, Error> {
        self.package_cache.get_all(
            || fs::read_dir(self.path())?
                .filter_map(|entry| {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(e) => return Some(e)
                    };
                    let name = entry.file_name().into_string()
                        .ok_or(ErrorKind::InvalidLocalPackage(entry.file_name().to_string_lossy()))?;
                    Some(Ok(entry.file_name()))
                }),
            |name| get_package(self.path(), name)
                .context(ErrorKind::InvalidLocalPackage(name.to_owned()))
        )
    }
}

/// A package database.
#[derive(Debug)]
pub struct LocalDatabaseInner {
    /// The level of signature verification required to accept packages
    sig_level: SignatureLevel,
    /// Which operations this database will be used for.
    usage: DbUsage,
    /// The database path.
    path: PathBuf,
    /// The package cache (HashMap of package name to package, which lazily gets info from disk)
    package_cache: HashMap<String, Rc<LocalDbPackage>>,
    /// The status as discovered during creation
    status: DbStatus,
}

impl LocalDatabaseInner {

    /// Helper to create a new database
    ///
    /// Path is the root path for databases.
    ///
    /// The database folder will be read to get a cache of package names.
    pub(crate) fn new(mut path: PathBuf, sig_level: SignatureLevel)
        -> Result<LocalDatabase, Error>
    {
        //  path is `$db_path SEP $local_db_name` for local
        path.push(LOCAL_DB_NAME);
        let mut db = LocalDatabase {
            sig_level,
            usage: DbUsage::default(),
            path,
            package_cache: HashMap::new(),
            read_package_folder: false,
        };
        db.populate_package_cache()?;
        Ok(db)
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
    fn package(&mut self, name: impl AsRef<str>) -> Result<Rc<LocalDbPackage>, Error> {
        let name = name.as_ref();
        self.package_cache.get(name)
            .ok_or(ErrorKind::InvalidLocalPackage(name.to_owned()))
    }

    /// Get the status of this database.
    fn status(&self) -> Result<DbStatus, Error> {
        let md = match fs::metadata(self.path()) {
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
        let version_path = self.path.join(LOCAL_DB_VERSION_FILE);
        let valid = match fs::read(&version_path) {
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
                        None => match LocalDatabase::create_version_file(&version_path) {
                            Ok(_) => true,
                            Err(e) => {
                                error!("could not create version file for local database at {}",
                                       self.path().display());
                                error!("caused by {}", e);
                                false
                            }
                        }
                    }
                    Err(e) => {
                        error!("could not check contents of local database directory at {}",
                               self.path().display());
                        error!("caused by {}", e);
                        false
                    }
                }
            },
            Err(e) => {
                error!("could not read version file for the local database at {}",
                       self.path().display());
                error!("caused by {}", e);
                false
            }
        };
        Ok(DbStatus::Exists { valid })
    }


    /// Load all package names into the cache, and validate the database
    fn populate_package_cache(&mut self) -> Result<(), Error> {
        debug!(r#"searching for local packages in "{}""#, self.path().display());
        for entry in fs::read_dir(self.path())? {
            let entry = entry?;
            if ! entry.metadata()?.is_dir() {
                // Check ALPM_DB_VERSION
                if entry.file_name() == LOCAL_DB_VERSION_FILE {
                } else {
                    // ignore extra files for now (should probably error)
                    continue;
                }
            }
            let path = entry.path();
            // Non-utf8 is hard until https://github.com/rust-lang/rfcs/pull/2295 lands
            let file_name = entry.file_name()
                .into_string()
                .expect("non-utf8 package names not yet supported");
            let (name, version) = split_package_dirname(&file_name)
                .ok_or(ErrorKind::InvalidLocalPackage(file_name.to_owned()))?;
            debug!(r#"found "{}", version: "{}""#, name, version);
            //let package_raw = fs::read_to_string(path.join("desc"))?;
            //let package: Package = de::from_str(&package_raw)
            //    .context(ErrorKind::InvalidLocalPackage(file_name.to_owned()))?;
            self.package_cache.insert(name.to_owned(),
                                      Rc::new(LocalDbPackage::new(path.to_owned())));
        }
        Ok(())
    }
}

/// Get a package from the given path with the given name in the localdb format.
fn get_package(package_dir: &Path, name: &str) -> Result<Package, failure::Error> {
    let filename = fs::read_dir(package_dir)
        .map(|entry| entry.file_name().into_string().expect("package names must be utf8"))
        .find(|&filename| filename.starts_with(name))
        .ok_or(err_msg("cannot find package"))?;
    let (name, version) = split_package_dirname(&filename)
       .ok_or(format_err!("cannot parse package dir name: {}", &filename))?;
    debug!("Processing package {}", name);

    let mut package_dir = package_dir.to_owned();
    package_dir.push(filename);
    package_dir.push("desc");
    let package_raw = fs::read_to_string(package_dir))?;
    let package: Package = de::from_str(&package_raw)?;
    // todo check name and version match
    Ok(package)

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
