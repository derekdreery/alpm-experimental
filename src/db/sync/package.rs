use failure::{format_err, ResultExt};
use serde_derive::{Deserialize, Serialize};

use crate::alpm_desc::de;
use crate::error::{Error, ErrorKind};
use crate::package::Package;
use derivative::Derivative;

/// A package from a sync database.
#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq, Hash)]
pub struct SyncPackage {
    desc: SyncPackageDescription,
}

impl SyncPackage {
    pub(crate) fn from_parts(desc_raw: &str, name: &str, version: &str) -> Result<Self, Error> {
        // get package description
        let desc: SyncPackageDescription =
            de::from_str(&desc_raw).context(ErrorKind::InvalidSyncPackage(name.to_owned()))?;

        // check package name/version with path
        if desc.name != name {
            return Err(format_err!(
                r#"Name on system ("{}") does not match name in package ("{}")"#,
                name,
                desc.name
            )
            .context(ErrorKind::InvalidSyncPackage(name.to_owned()))
            .into());
        }
        if desc.version != version {
            return Err(format_err!(
                r#"Version on system ("{}") does not match version in package ("{}")"#,
                version,
                desc.version
            )
            .context(ErrorKind::InvalidSyncPackage(name.to_owned()))
            .into());
        }

        Ok(SyncPackage { desc })
    }
}

impl Package for SyncPackage {
    fn name(&self) -> &str {
        &self.desc.name
    }

    fn version(&self) -> &str {
        &self.desc.version
    }

    fn base(&self) -> Option<&str> {
        self.desc.base.as_ref().map(|v| v.as_ref())
    }

    fn description(&self) -> &str {
        &self.desc.description
    }

    fn groups(&self) -> &[String] {
        &self.desc.groups
    }

    fn url(&self) -> Option<&str> {
        self.desc.url.as_ref().map(|s| s.as_str())
    }

    fn license(&self) -> &[String] {
        &self.desc.license
    }

    fn arch(&self) -> &str {
        &self.desc.arch
    }

    fn build_date(&self) -> &str {
        &self.desc.build_date
    }

    fn packager(&self) -> &str {
        &self.desc.packager
    }

    fn size(&self) -> u64 {
        self.desc.installed_size
    }

    fn replaces(&self) -> &[String] {
        &self.desc.replaces
    }

    fn depends(&self) -> &[String] {
        &self.desc.depends
    }

    fn optional_depends(&self) -> &[String] {
        &self.desc.optional_depends
    }

    fn make_depends(&self) -> &[String] {
        &self.desc.make_depends
    }

    fn check_depends(&self) -> &[String] {
        &self.desc.check_depends
    }

    fn conflicts(&self) -> &[String] {
        &self.desc.conflicts
    }

    fn provides(&self) -> &[String] {
        &self.desc.provides
    }
}

/// Struct to help deserializing `desc` file
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SyncPackageDescription {
    pub(crate) filename: String,
    pub(crate) name: String,
    pub(crate) base: Option<String>,
    pub(crate) version: String,
    #[serde(rename = "desc")]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) groups: Vec<String>,
    #[serde(rename = "csize")]
    pub(crate) compressed_size: u64,
    #[serde(rename = "isize")]
    pub(crate) installed_size: u64,
    pub(crate) md5sum: String,
    pub(crate) sha256sum: String,
    #[serde(rename = "pgpsig")]
    pub(crate) pgp_signature: String,
    pub(crate) url: Option<String>,
    #[serde(default)]
    pub(crate) license: Vec<String>,
    pub(crate) arch: String,
    #[serde(rename = "builddate")]
    pub(crate) build_date: String,
    pub(crate) packager: String,
    #[serde(default)]
    pub(crate) replaces: Vec<String>,
    #[serde(default)]
    pub(crate) depends: Vec<String>,
    #[serde(rename = "optdepends")]
    #[serde(default)]
    pub(crate) optional_depends: Vec<String>,
    #[serde(rename = "makedepends")]
    #[serde(default)]
    pub(crate) make_depends: Vec<String>,
    #[serde(rename = "checkdepends")]
    #[serde(default)]
    pub(crate) check_depends: Vec<String>,
    #[serde(default)]
    pub(crate) conflicts: Vec<String>,
    #[serde(default)]
    pub(crate) provides: Vec<String>,
}
