use chrono::NaiveDateTime;
use failure::ResultExt;
use gpgme::{self, Protocol};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use error::{Error, ErrorKind};
use Alpm;

enum SignatureStatus {
    Valid,
    KeyExpired,
    SignatureExpired,
    KeyUnknown,
    KeyDisabled,
    Invalid,
}

enum SignatureValidity {
    Full,
    Marginal,
    Never,
    Unknown,
}

struct GpgKey {
    fingerprint: String,
    uid: String,
    name: String,
    email: String,
    created: NaiveDateTime,
    expires: NaiveDateTime,
    length: usize,
    revoked: bool,
    algorithm: u8,
}

struct SigResult {
    key: GpgKey,
    status: SignatureStatus,
    validity: SignatureValidity,
}

type SigList = Vec<SigResult>;

const SIG_EXTENSION: &str = ".sig";

/// Get the path of a signature from the path of a file (append ".sig").
fn alpm_sigpath(path: &Path) -> Option<PathBuf> {
    path.file_name().map(|name| {
        let mut name = name.to_owned();
        name.push(SIG_EXTENSION);
        path.with_file_name(name)
    })
}

pub fn init(gpg_directory: impl AsRef<Path>) -> Result<(), Error> {
    let gpg_directory = gpg_directory.as_ref();
    let pub_ring = gpg_directory.join("pubring.gpg");
    let trustdb = gpg_directory.join("trustdb.gpg");
    let gpg_directory_str = gpg_directory.to_str()
        .ok_or(Error::from(ErrorKind::Gpgme))?;

    // Setup gpg
    let gpg_handle = gpgme::init();
    debug!("using gpg version {}", gpg_handle.version());
    gpg_handle.check_engine_version(Protocol::OpenPgp)
        .context(ErrorKind::Gpgme)?;
    // Set protocol, path, and home dir
    let none_type_helper: Option<String> = None;
    gpg_handle.set_engine_info(Protocol::OpenPgp, none_type_helper, Some(gpg_directory_str))
        .context(ErrorKind::Gpgme)?;
    let engine_infos = gpg_handle.engine_info()
        .context(ErrorKind::Gpgme)?;
    debug!("gpg engine info:");
    for engine_info in engine_infos.iter() {
        let protocol = match engine_info.protocol().name_raw() {
            Some(ref s) => s.to_string_lossy(),
            None => break
        };
        debug!("-- {} --", protocol);
        debug!("path: {:?}", engine_info.path_raw().map(|s| s.to_string_lossy()));
        debug!("home dir: {:?}", engine_info.home_dir_raw().map(|s| s.to_string_lossy()));
        debug!("version: {:?}", engine_info.version_raw().map(|s| s.to_string_lossy()));
    };

    Ok(())
}

/// Check the signature of a file
fn check_signature(path: &Path, signature: &[u8]) -> Result<(), Error> {
    let path_str = path.to_string_lossy().into_owned();
    if ! path.is_file() {
        let path_str = path.to_string_lossy().into_owned();
        return Err(format_err!(r#""{}" is not a file"#, path_str)
            .context(ErrorKind::UnexpectedSignature(path_str)).into());
    }
    let file = File::open(path)
        .context(ErrorKind::UnexpectedSignature(path_str.clone()))?;
    let gpg_ctx = gpgme::Context::from_protocol(Protocol::OpenPgp)
        .context(ErrorKind::UnexpectedSignature(path_str.clone()))?;
    let data = gpgme::Data::from_seekable_reader(file)
        .context(ErrorKind::UnexpectedSignature(path_str.clone()))?;
    Ok(())
}