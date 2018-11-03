/// Information that is available on all packages - regardless of their location.
pub trait Package {
    /// The package name.
    fn name(&self) -> &str;

    /// The package version.
    fn version(&self) -> &str;

    /// The base of this package.
    fn base(&self) -> Option<&str>;

    /// The package description.
    fn description(&self) -> &str;

    /// The groups this package is in.
    fn groups(&self) -> &[String];

    /// The url for this package.
    fn url(&self) -> &str;

    /// The license for this package, if any.
    fn license(&self) -> Option<&str>;

    /// The computer architecture this package is compiled for.
    fn arch(&self) -> &str;

    /// The person who created this package
    fn packager(&self) -> &str;

    /// The reason this package was installed, if given.
    fn reason(&self) -> Option<Reason>;

    /// The available types of validation for this package.
    fn validation(&self) -> &[Validation];

    /// The size in bytes of this package.
    fn size(&self) -> u64;

    /// Which packages this package replaces.
    fn replaces(&self) -> &[String];

    /// Which packages this package depends on.
    fn depends(&self) -> &[String];

    /// Which packages this package optionally depends on.
    fn optional_depends(&self) -> &[String];

    /// Which packages this package conflicts with.
    fn conflicts(&self) -> &[String];

    /// Which virtual packages this package provides.
    fn provides(&self) -> &[String];
}

/// Struct to help deserializing `desc` file
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub(crate) struct PackageDescription {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) base: Option<String>,
    #[serde(rename = "desc")]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) groups: Vec<String>,
    pub(crate) url: String,
    pub(crate) license: Option<String>,
    pub(crate) arch: String,
    //build_date: SystemTime,
    //install_date: SystemTime,
    pub(crate) packager: String,
    pub(crate) reason: Option<Reason>,
    pub(crate) validation: Vec<Validation>,
    pub(crate) size: u64,
    #[serde(default)]
    pub(crate) replaces: Vec<String>,
    #[serde(default)]
    pub(crate) depends: Vec<String>,
    #[serde(rename = "optdepends")]
    #[serde(default)]
    pub(crate) optional_depends: Vec<String>,
    #[serde(default)]
    pub(crate) conflicts: Vec<String>,
    #[serde(default)]
    pub(crate) provides: Vec<String>,
}

/// Different possible validation methods
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum Reason {
    /// This package was explicitally installed
    #[serde(rename = "0")]
    Explicit,
    /// This package was installed because it was required for another package
    #[serde(rename = "1")]
    Depend,
}
