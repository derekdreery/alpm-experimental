#![feature(nll)]

//! This example is unix-only

#[cfg(not(unix))]
compile_error!("This example is unix only");

extern crate alpm;
extern crate env_logger;
extern crate failure;
extern crate log;
extern crate users;

use alpm::{Alpm, Error};
use alpm::db::LocalDbPackage as Package;
use failure::Fail;
use log::LevelFilter;


use std::fs;
use std::path::Path;
use std::process::Command;

const BASE_PATH: &str = "/tmp/alpm-test";

fn run() -> Result<(), Error> {
    let alpm = Alpm::new()
        //.with_root_path(&BASE_PATH)
        .build()?;

    /*
    alpm.register_sync_database("core")?;
    alpm.register_sync_database("extra")?;
    alpm.register_sync_database("community")?;
    alpm.register_sync_database("multilib")?;
    */

    print_packages_with_no_reason(&alpm)?;
    print_total_package_size(&alpm)?;

    /*
    let mut core = alpm.sync_database("core")?;
    core.add_server(server_url("core", "x86_64"))?;
    println!(r#"core db ("{}") status: {:?}"#, core.path().display(), core.status()?);
    core.synchronize(false)?;

    let mut extra = alpm.sync_database("extra")?;
    extra.add_server(server_url("extra", "x86_64"))?;
    println!(r#"core db ("{}") status: {:?}"#, core.path().display(), core.status()?);
    extra.synchronize(false)?;

    extra.add_server(&server_url("extra", "x86_64"))?;
    community.add_server(&server_url("community", "x86_64"))?;
    multilib.add_server(&server_url("multilib", "x86_64"))?;
    */

    Ok(())
}

fn main() {
    // Make a temporary archlinux installation.
    //make_base();

    // Make logging nice
    let mut builder = env_logger::Builder::from_default_env();
    builder
        .filter_level(LevelFilter::Debug)
        .filter_module("tokio_reactor", LevelFilter::Warn)
        .filter_module("tokio_core", LevelFilter::Warn)
        .filter_module("hyper", LevelFilter::Warn)
        .init();

    if let Err(e) = run() {
        let mut causes = e.causes();
        println!("-- Error --");
        let first = causes.next().unwrap();
        println!("{}", first);
        let mut backtrace = first.backtrace();
        for cause in causes {
            println!("  caused by: {}", cause);
            if let Some(bt) = cause.backtrace() {
                backtrace = Some(bt);
            }
        }
        if let Some(bt) = backtrace {
            println!("-- Backtrace --");
            println!("{}", bt);
        }
    }
}

/// Just makes a valid server url for given database/os.
fn server_url(database: impl AsRef<str>, os: impl AsRef<str>) -> String {
    format!("http://mirror.bytemark.co.uk/archlinux/{}/os/{}", database.as_ref(), os.as_ref())
}

/// Make a directory with a base installation at /tmp/alpm-test
fn make_base() {

    let base_path = Path::new(BASE_PATH);
    if base_path.is_file() {
        fs::remove_file(base_path).unwrap();
    }
    if ! base_path.exists() {
        let user = users::get_current_username().unwrap();
        let group = users::get_current_groupname().unwrap();

        fs::create_dir_all(BASE_PATH).unwrap();
        let mut cmd = Command::new("sudo");
        cmd.args(&["pacstrap", BASE_PATH, "base"]);
        if ! run_command(cmd) {
            cleanup_and_fail();
        }
        let mut chown = Command::new("sudo");
        chown.arg("chown")
            .arg("-R")
            .arg(format!("{}:{}", user, group))
            .arg(BASE_PATH);
        if ! run_command(chown) {
            cleanup_and_fail();
        }
    }
}

/// Remove tmp dir and panic
fn cleanup_and_fail() {
    assert!(BASE_PATH.starts_with("/tmp")); // don't destroy stuff
    fs::remove_dir_all(BASE_PATH).unwrap();
    panic!("make_base failed");
}

/// Run a command and panic on bad exit status
fn run_command(mut cmd: Command) -> bool {
    use std::process::Stdio;
    cmd.stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = cmd.status().unwrap();
    if status.success() {
        true
    } else {
        eprintln!("command {:?} failed with error code {:?}", cmd, status.code());
        false
    }
}

/// Print all packages, and their disk usage, where packages have no reason field.
fn print_packages_with_no_reason(alpm: &Alpm) -> Result<(), Error> {
    let local_db = alpm.local_database();
    println!("-- Packages without install reason --");
    let mut packages = local_db.packages()
        .map(|pkg| pkg.unwrap())
        .filter(|pkg| pkg.reason().is_none())
        .map(|pkg| {
            let disk_usage = pkg.disk_usage(&*alpm.root_path())?;
            Ok((pkg, disk_usage))
        })
        .collect::<Result<Vec<(Package, u64)>, Error>>()?;
   
    packages.sort_by_key(|&(_, usage)| usage);
    let mut acc = 0;
    for (pkg, usage) in packages.iter() {
        println!("{}: {}", pkg.name(), usage);
        acc += usage;
    }
    println!("Total disk space from packages without install reason: {}", acc);
    Ok(())
}

/// Print the total disk usage of all local packages
fn print_total_package_size(alpm: &Alpm) -> Result<(), Error> {
    let local_db = alpm.local_database();
    let total_usage = local_db.packages()
        .fold(Ok(0), |acc: Result<u64, Error>, pkg| {
            match acc {
                Ok(val) => Ok(val + pkg?.disk_usage(&*alpm.root_path())?),
                Err(e) => Err(e),
            }
        })?;
   
    println!("Total disk space from packages: {}", total_usage);
    Ok(())
}
