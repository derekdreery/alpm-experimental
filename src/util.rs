use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

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
