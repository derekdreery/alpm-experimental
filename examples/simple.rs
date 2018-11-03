#![feature(nll)]

//! This example is unix-only

#[cfg(not(unix))]
compile_error!("This example is unix only");

extern crate alpm;
#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate failure;
extern crate humansize;
extern crate log;
extern crate progress;
extern crate users;

use alpm::db::Database;
use alpm::{Alpm, Error, Package};
use clap::{App, AppSettings, Arg, ArgMatches};
use failure::Fail;
use humansize::{file_size_opts::BINARY, FileSize};
use log::LevelFilter;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

const BASE_PATH: &str = "/tmp/alpm-test";

/// Command line arguments parsed into program config.
#[derive(Debug)]
pub struct Opts {
    /// How verbose should we be?
    pub verbosity: LevelFilter,
    /// Which subcommand should we run?
    pub subcommand: Cmd,
}

/// Which subcommand to run
#[derive(Debug)]
pub enum Cmd {
    /// Generate a disk usage report
    DiskUsageReport {
        /// Whether sizes are in human-readable form
        human: bool,
    },
    /// Validate all packages
    Validate,
}

fn run(opts: Opts) -> Result<(), Error> {
    let alpm = Alpm::new()
        //.with_root_path(&BASE_PATH)
        .build()?;

    /*
    alpm.register_sync_database("core")?;
    alpm.register_sync_database("extra")?;
    alpm.register_sync_database("community")?;
    alpm.register_sync_database("multilib")?;
    */

    match opts.subcommand {
        Cmd::DiskUsageReport { human: _ } => {
            let local_db = alpm.local_database();
            let mut reported_size = 0;
            let mut size_on_disk = 0;
            let mut idx = 0;
            let total = local_db.count();
            let mut bar = progress::Bar::new();

            local_db.packages(|pkg| -> Result<(), Error> {
                let title = format!("Pkg {} of {} ({}) ", idx + 1, total, pkg.name());
                bar.set_job_title(&shorten_ellipsis(&title, 40));
                reported_size += pkg.size();
                size_on_disk += pkg.size_on_disk()?;
                idx += 1;
                bar.reach_percent(((idx * 100) / total) as i32);
                // bail early
                /*
                if idx > 100 {
                    return Err(alpm::ErrorKind::UseAfterDrop.into());
                }
                */
                Ok(())
            })?;

            println!(
                "Reported size: {}",
                reported_size.file_size(BINARY).unwrap()
            );
            println!("Actual size: {}", size_on_disk.file_size(BINARY).unwrap());
        }
        Cmd::Validate => {
            let local_db = alpm.local_database();

            let mut errors = HashMap::with_capacity(local_db.count());
            local_db.packages(|pkg| -> Result<(), Error> {
                let pkg_errors = pkg.validate()?;
                if pkg_errors.len() > 0 {
                    errors.insert(pkg.name().to_owned(), pkg_errors);
                }
                Ok(())
            })?;
            for (name, errs) in errors {
                println!("--{}--", name);
                for err in errs {
                    println!("  {}", err);
                }
            }
        }
    }

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

/// Print all packages, and their disk usage, where packages have no reason field.
fn print_packages_with_no_reason(alpm: &Alpm) -> Result<(), Error> {
    let local_db = alpm.local_database();
    let mut packages = Vec::new();
    local_db.packages(|pkg| -> Result<(), Error> {
        if pkg.reason().is_none() {
            packages.push(pkg.clone());
        }
        Ok(())
    })?;

    packages.sort_by(|a, b| a.name().cmp(b.name()));
    let mut acc = 0;
    let mut iter = packages.iter();
    println!("-- Packages without install reason --");
    if let Some(pkg) = iter.next() {
        print!("{}", pkg.name());
        acc += pkg.size();
    }
    for pkg in iter {
        print!(", {}", pkg.name());
        acc += pkg.size();
    }
    println!();
    println!(
        "Total disk space from packages without install reason: {}",
        acc.file_size(BINARY).unwrap()
    );
    Ok(())
}

/// Print the total disk usage of all local packages
fn print_total_package_size(alpm: &Alpm) -> Result<(), Error> {
    let local_db = alpm.local_database();
    let mut total_usage = 0;
    local_db.packages(|pkg| -> Result<(), Error> {
        total_usage += pkg.size();
        Ok(())
    })?;

    println!(
        "Total disk space from packages: {}",
        total_usage.file_size(BINARY).unwrap()
    );
    Ok(())
}

impl Opts {
    fn from_args<'a>(matches: ArgMatches<'a>) -> Opts {
        Opts {
            verbosity: match matches.occurrences_of("verbosity") {
                0 => LevelFilter::Warn,
                1 => LevelFilter::Info,
                _ => LevelFilter::Debug,
            },
            subcommand: Cmd::from_args(matches),
        }
    }
}

impl Cmd {
    fn from_args<'a>(matches: ArgMatches<'a>) -> Cmd {
        match matches.subcommand() {
            ("disk", Some(sub_m)) => Cmd::DiskUsageReport {
                human: sub_m.is_present("human"),
            },
            ("validate", Some(_sub_m)) => Cmd::Validate,
            _ => unreachable!(),
        }
    }
}

fn main() {
    // Make a temporary archlinux installation.
    //make_base();

    // Do argument parsing
    let args = App::new("simple")
        .author(crate_authors!())
        .version(crate_version!())
        .about("A command line tool showing off some of the functionality of the library.")
        .setting(AppSettings::SubcommandRequired)
        .arg(
            Arg::with_name("verbosity")
                .long("verbose")
                .short("v")
                .multiple(true)
                .help("how verbose to be when logging"),
        )
        .subcommand(
            App::new("disk").about("Prints a disk-usage report.").arg(
                Arg::with_name("human")
                    .short("r")
                    .long("human-readable")
                    .help("if present, disk sized will be in human-readable form"),
            ),
        )
        .subcommand(App::new("validate").about("Check all packages against the local database."))
        .get_matches();
    let opts = Opts::from_args(args);

    // Make logging nice
    let mut builder = env_logger::Builder::from_default_env();
    builder
        .filter_level(LevelFilter::Warn)
        .filter_module("alpm", opts.verbosity)
        .filter_module("simple", opts.verbosity)
        .init();

    if let Err(e) = run(opts) {
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
    format!(
        "http://mirror.bytemark.co.uk/archlinux/{}/os/{}",
        database.as_ref(),
        os.as_ref()
    )
}

/// Make a directory with a base installation at /tmp/alpm-test
fn make_base() {
    let base_path = Path::new(BASE_PATH);
    if base_path.is_file() {
        fs::remove_file(base_path).unwrap();
    }
    if !base_path.exists() {
        let user = users::get_current_username().unwrap();
        let group = users::get_current_groupname().unwrap();

        fs::create_dir_all(BASE_PATH).unwrap();
        let mut cmd = Command::new("sudo");
        cmd.args(&["pacstrap", BASE_PATH, "base"]);
        if !run_command(cmd) {
            cleanup_and_fail();
        }
        let mut chown = Command::new("sudo");
        chown
            .arg("chown")
            .arg("-R")
            .arg(format!("{}:{}", user, group))
            .arg(BASE_PATH);
        if !run_command(chown) {
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
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    let status = cmd.status().unwrap();
    if status.success() {
        true
    } else {
        eprintln!(
            "command {:?} failed with error code {:?}",
            cmd,
            status.code()
        );
        false
    }
}

/// Take some text and shorten it
fn shorten_ellipsis<'a>(input: &'a str, len: usize) -> Cow<'a, str> {
    if input.len() > len {
        let mut new_len = len - 4;
        while !input.is_char_boundary(new_len) {
            new_len -= 1;
        }
        Cow::Owned(format!("{} ...", &input[0..new_len]))
    } else {
        Cow::Borrowed(input)
    }
}
