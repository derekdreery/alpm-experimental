use std::path::{PathBuf, Path};
use std::fs;
use std::fmt;
use std::io;
use std::marker::PhantomData;
use std::time::SystemTime;
use std::collections::HashMap;

use failure::ResultExt;
use libarchive;

use alpm_desc::de;
use error::{Error, ErrorKind};

#[derive(Debug)]
pub struct Package {
    pub path: PathBuf,
    desc: PackageDesc,
    files: Vec<PathBuf>,
}

impl Package {
    pub fn from_local(path: PathBuf, name: &str, version: &str) 
        -> Result<Package, Error> 
    {
        // get package description
        let desc_raw = fs::read_to_string(path.join("desc"))?;
        let desc: PackageDesc = de::from_str(&desc_raw)
            .context(ErrorKind::InvalidLocalPackage(name.to_owned()))?;

        // check package name/version with path
        if desc.name != name {
            return Err(format_err!(r#"Name on system ("{}") does not match name in package ("{}")"#, name, desc.name)
            .context(ErrorKind::InvalidLocalPackage(name.to_owned())).into());
        }
        if desc.version != version {
            return Err(format_err!(r#"Version on system ("{}") does not match version in package ("{}")"#, version, desc.version)
            .context(ErrorKind::InvalidLocalPackage(name.to_owned())).into());

        }

        // get files
        let files_raw = fs::read_to_string(path.join("files"))?;
        let files: Files = de::from_str(&files_raw)
            .context(ErrorKind::InvalidLocalPackage(name.to_owned()))?;

        // get mtree
        

        Ok(Package { path, desc, files: files.files })
    }

    /// The package name
    pub fn name(&self) -> &str {
        &self.desc.name
    }

    /// The package version
    pub fn version(&self) -> &str {
        &self.desc.version
    }
    
    pub fn base(&self) -> Option<&str> {
        match self.desc.base {
            Some(ref b) => Some(b),
            None => None
        }
    }
    
    pub fn description(&self) -> &str {
        &self.desc.description
    }

    pub fn groups(&self) -> &[String] {
        &self.desc.groups
    }

    pub fn url(&self) -> &str {
        &self.desc.url
    }

    pub fn license(&self) -> &str {
        &self.desc.license
    }

    pub fn arch(&self) -> &str {
        &self.desc.arch
    }

    pub fn packager(&self) -> &str {
        &self.desc.packager
    }

    pub fn reason(&self) -> Option<Reason> {
        self.desc.reason
    }

    pub fn validation(&self) -> &[Validation] {
        &self.desc.validation
    }

    pub fn size(&self) -> u64 {
        self.desc.size
    }

    pub fn replaces(&self) -> &[String] {
        &self.desc.replaces
    }

    pub fn depends(&self) -> &[String] {
        &self.desc.depends
    }

    pub fn optional_depends(&self) -> &[String] {
        &self.desc.optional_depends
    }

    pub fn conflicts(&self) -> &[String] {
        &self.desc.conflicts
    }

    pub fn provides(&self) -> &[String] {
        &self.desc.provides
    }

    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Helper function that reads the size of all files on disk
    ///
    /// If the file is not found, it is skipped, otherwise the error is raised
    pub fn disk_usage(&self, root: impl AsRef<Path>) -> Result<u64, io::Error> {
        let root = root.as_ref();
        let mut acc = 0;
        for file in self.files() {
            match fs::metadata(root.join(file)) {
                Ok(md) => {
                    if md.is_file() {
                        acc += md.len();
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                    warn!("File {} in installed package {} is missing on disk",
                          root.join(file).display(),
                          self.name());
                },
                Err(e) => return Err(e)
            }
        }
        Ok(acc)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PackageDesc {
    name: String,
    version: String,
    base: Option<String>,
    #[serde(rename = "desc")]
    description: String,
    #[serde(default)]
    groups: Vec<String>,
    url: String,
    license: String,
    arch: String,
    //build_date: SystemTime,
    //install_date: SystemTime,
    packager: String,
    reason: Option<Reason>,
    validation: Vec<Validation>,
    size: u64,
    #[serde(default)]
    replaces: Vec<String>,
    #[serde(default)]
    depends: Vec<String>,
    #[serde(rename = "optdepends")]
    #[serde(default)]
    optional_depends: Vec<String>,
    #[serde(default)]
    conflicts: Vec<String>,
    #[serde(default)]
    provides: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Files {
    #[serde(default)]
    files: Vec<PathBuf>
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Validation {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "md5")]
    Md5,
    #[serde(rename = "sha256")]
    Sha256,
    #[serde(rename = "pgp")]
    Pgp,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub enum Reason {
    /// This package was explicitally installed
    #[serde(rename = "0")]
    Explicit,
    /// This package was installed because it was required for another package
    #[serde(rename = "1")]
    Depend,
}

fn get_mtree(path: impl AsRef<Path>) -> libarchive::error::ArchiveResult<()> {
    let mut builder = libarchive::reader::Builder::new();
    builder.support_filter(libarchive::archive::ReadFilter::All)?;
    builder.support_format(libarchive::archive::ReadFormat::Mtree)?;
    let mut file = builder.open_file(path)?;
    let mut entry = builder.entry();
    OK(())
}
