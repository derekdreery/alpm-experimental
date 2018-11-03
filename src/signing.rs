// todo I need to think more about whether we can just use types from gpgme more.
use failure::{err_msg, Fail, ResultExt};
use gpgme::{self, KeyAlgorithm, Protocol};
use std::ffi::OsString;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::{Error, ErrorKind};

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
    created: SystemTime,
    expires: SystemTime,
    length: usize,
    revoked: bool,
    algorithm: KeyAlgorithm,
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
    let gpg_directory_str = gpg_directory
        .to_str()
        .ok_or(Error::from(ErrorKind::Gpgme))?;

    // Setup gpg
    let gpg_handle = gpgme::init();
    debug!("using gpg version {}", gpg_handle.version());
    gpg_handle
        .check_engine_version(Protocol::OpenPgp)
        .context(ErrorKind::Gpgme)?;
    // Set protocol, path, and home dir
    // We need this to keep the type checker happy.
    let none_type_helper: Option<String> = None;
    gpg_handle
        .set_engine_info(Protocol::OpenPgp, none_type_helper, Some(gpg_directory_str))
        .context(ErrorKind::Gpgme)?;
    let engine_infos = gpg_handle.engine_info().context(ErrorKind::Gpgme)?;
    debug!("gpg engine info:");
    for engine_info in engine_infos.iter() {
        let protocol = match engine_info.protocol().name_raw() {
            Some(ref s) => s.to_string_lossy(),
            None => break,
        };
        debug!("-- {} --", protocol);
        debug!(
            "path: {:?}",
            engine_info.path_raw().map(|s| s.to_string_lossy())
        );
        debug!(
            "home dir: {:?}",
            engine_info.home_dir_raw().map(|s| s.to_string_lossy())
        );
        debug!(
            "version: {:?}",
            engine_info.version_raw().map(|s| s.to_string_lossy())
        );
    }

    Ok(())
}

/// Takes the path to a file and a detached signature, and returns a `gpgme::VerificationResult`,
/// a list of found signatures (with some extra context).
///
/// If the signature is not supplied it is assumed to be with the file with a ".sig" suffix.
fn verify_signatures<T>(path: &Path, signature: Option<&T>) -> Result<Vec<GpgKey>, Error>
where
    T: AsRef<[u8]> + ?Sized,
{
    let path_str = path.to_string_lossy().into_owned();
    if !path.is_file() {
        let path_str = path.to_string_lossy().into_owned();
        return Err(format_err!(r#""{}" is not a file"#, path_str)
            .context(ErrorKind::UnexpectedSignature(path_str))
            .into());
    }
    let mut gpg_ctx = gpgme::Context::from_protocol(Protocol::OpenPgp)
        .context(ErrorKind::UnexpectedSignature(path_str.clone()))?;
    let file = File::open(path).context(ErrorKind::UnexpectedSignature(path_str.clone()))?;
    // todo add error context when gpgme releases next version.
    let data = match gpgme::Data::from_seekable_reader(file) {
        Ok(d) => d,
        Err(_) => return Err(ErrorKind::UnexpectedSignature(path_str.clone()).into()),
    };
    let signature = match signature {
        Some(ref buf) => gpgme::Data::from_buffer(buf)
            .context(ErrorKind::UnexpectedSignature(path_str.clone()))?,
        None => {
            // we already know we have a file
            let file_path = path.file_name().unwrap();
            let mut sig_file_path = OsString::with_capacity(file_path.len() + SIG_EXTENSION.len());
            sig_file_path.push(file_path);
            sig_file_path.push(SIG_EXTENSION);
            let sig_path = path.with_file_name(sig_file_path);
            let sig_file =
                File::open(sig_path).context(ErrorKind::UnexpectedSignature(path_str.clone()))?;
            match gpgme::Data::from_seekable_reader(sig_file) {
                Ok(d) => d,
                Err(_) => return Err(ErrorKind::UnexpectedSignature(path_str.clone()).into()),
            }
        }
    };
    let result = gpg_ctx
        .verify_detached(signature, data)
        .context(ErrorKind::UnexpectedSignature(path_str.clone()))?;
    result
        .signatures()
        .enumerate()
        .map(|(idx, sig)| {
            debug!("-- signature {} --", idx);
            debug!("summary: {:?}", sig.summary());
            match sig.status() {
                Ok(_) => debug!("status: good"),
                Err(e) => debug!("status: {}", e),
            };
            if let Some(created) = sig.creation_time() {
                debug!("created: {:?}", created);
                if created > SystemTime::now() {
                    warn!("key timestamp for created at is in the future");
                }
            } else {
                warn!("no creation timestamp in key");
            }
            if let Some(expires) = sig.expiration_time() {
                debug!("expires: {:?}", expires);
            } else {
                debug!("expires: never");
            }
            debug!("validity: {}", sig.validity());
            if let Some(reason) = sig.nonvalidity_reason() {
                debug!("nonvalidity reason: {}", reason);
            }
            Ok(match sig.key() {
                Some(key) => {
                    let fingerprint = key.fingerprint().or_else(|e| match e {
                        Some(err) => {
                            Err(err.context(ErrorKind::UnexpectedSignature(path_str.clone())))
                        }
                        None => sig.fingerprint().map_err(|e| match e {
                            Some(err) => {
                                err.context(ErrorKind::UnexpectedSignature(path_str.clone()))
                            }
                            None => err_msg("fingerprint not found!")
                                .context(ErrorKind::UnexpectedSignature(path_str.clone())),
                        }),
                    })?;
                    debug!("fingerprint: {:?}", fingerprint);
                    // todo I'm getting bored of error handling
                    let user = key.user_ids().next().unwrap();
                    GpgKey {
                        fingerprint: fingerprint.to_owned(),
                        uid: user.id().unwrap().to_owned(),
                        name: user.name().unwrap().to_owned(),
                        email: user.email().unwrap().to_owned(),
                        created: sig.creation_time().unwrap(),
                        expires: sig.expiration_time().unwrap(),
                        length: 0,
                        revoked: user.is_revoked(),
                        algorithm: sig.key_algorithm(),
                    }
                }
                None => unimplemented!(),
            })
        })
        .collect()
}
