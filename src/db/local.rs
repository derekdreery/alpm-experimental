use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::cell::RefCell;

use atoi::atoi;
use failure::Fail;

use error::{ErrorKind, Error};
use package::Package;
use db::{LOCAL_DB_NAME, Database, SignatureLevel, DbStatus, DbUsage};

const LOCAL_DB_VERSION_FILE: &str = "ALPM_DB_VERSION";
const LOCAL_DB_CURRENT_VERSION: u64 = 9;

/// A package database.
#[derive(Debug)]
pub struct LocalDatabase {
    /// The level of signature verification required to accept packages
    sig_level: SignatureLevel,
    /// Which operations this database will be used for.
    usage: DbUsage,
    /// The database path.
    path: PathBuf,
    /// The package cache (HashMap of package name to package)
    package_cache: RefCell<HashMap<String, Package>>,
}

impl LocalDatabase {

    /// Helper to create a new database
    pub(crate) fn new(mut path: PathBuf, sig_level: SignatureLevel) -> LocalDatabase {
        //  path is `$db_path SEP $local_db_name` for local
        path.push(LOCAL_DB_NAME);
        LocalDatabase {
            sig_level,
            usage: DbUsage::default(),
            path,
            package_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Helper to create a new version file for the local database.
    #[inline]
    fn create_version_file(path: &Path) -> io::Result<()> {
        let mut version_file = fs::File::create(&path)?;
        // Format is number followed by single newline
        write!(version_file, "{}\n", LOCAL_DB_CURRENT_VERSION)?;
        Ok(())
    }


    /// Populate the package cache
    ///
    /// It is up to the caller to check that this database is local.
    fn populate(&self) -> Result<(), Error> {
        for entry in fs::read_dir(self.path())? {
            let entry = entry?;
            let md = entry.metadata()?;
            if ! md.is_dir() {
                continue;
            }
        }
        Ok(())
    }
}

impl Database for LocalDatabase {
    /// Get the name of this database
    fn name(&self) -> &str {
        LOCAL_DB_NAME
    }

    /// Get the path of the root file or directory for this database.
    fn path(&self) -> &Path {
        &self.path
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

    /// Get the packages in this database
    fn packages(&self) -> &HashMap<String, Package> {
        unimplemented!();
    }
}

