
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
    fn url(&self) -> Option<&str>;

    /// The licenses for this package.
    fn license(&self) -> &[String];

    /// The computer architecture this package is compiled for.
    fn arch(&self) -> &str;

    /// The date and time that this package was built.
    fn build_date(&self) -> &str;

    /// The person who created this package
    fn packager(&self) -> &str;

    /// The size in bytes of this package.
    fn size(&self) -> u64;

    /// Which packages this package replaces.
    fn replaces(&self) -> &[String];

    /// Which packages this package depends on.
    fn depends(&self) -> &[String];

    /// Which packages this package optionally depends on.
    fn optional_depends(&self) -> &[String];

    /// Which packages this package depends on during build.
    fn make_depends(&self) -> &[String];

    /// Which packages this package depends on when checking the build.
    fn check_depends(&self) -> &[String];

    /// Which packages this package conflicts with.
    fn conflicts(&self) -> &[String];

    /// Which virtual packages this package provides.
    fn provides(&self) -> &[String];
}
