use std::fmt;
use std::marker::PhantomData;
use std::time::SystemTime;
use std::collections::HashMap;

use serde::de::{self, Visitor};
/*
#[derive(Debug, Deserialize, Serialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub base: Option<String>,
    #[serde(rename = "desc")]
    pub description: String,
    #[serde(default)]
    pub groups: Vec<String>,
    pub url: String,
    pub license: String,
    pub arch: String,
    //build_date: SystemTime,
    //install_date: SystemTime,
    pub packager: String,
    pub reason: Option<Reason>,
    pub validation: Vec<Validation>,
    pub size: u64,
    #[serde(default)]
    pub replaces: Vec<String>,
    #[serde(default)]
    pub depends: Vec<String>,
    #[serde(rename = "optdepends")]
    #[serde(default)]
    pub optional_depends: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub provides: Vec<String>,
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

#[derive(Debug, Deserialize, Serialize)]
pub enum Reason {
    /// This package was explicitally installed
    #[serde(rename = "0")]
    Explicit,
    /// This package was installed because it was required for another package
    #[serde(rename = "1")]
    Depend,
}
*/
