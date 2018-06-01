extern crate alpm;
extern crate env_logger;
extern crate log;

use alpm::{Alpm, Error};

use log::LevelFilter;


fn main() -> Result<(), Error> {
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(LevelFilter::Debug)
        .filter_module("tokio_reactor", LevelFilter::Warn)
        .filter_module("tokio_core", LevelFilter::Warn)
        .init();


    let alpm = Alpm::new().build()?;
    Ok(())
}