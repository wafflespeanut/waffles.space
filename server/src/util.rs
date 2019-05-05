use chrono::{SecondsFormat, offset::Utc};
use env_logger::Builder;
use log::LevelFilter;

use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

/// Prepares the logger with the universal datetime format and INFO level.
pub fn prepare_logger() {
    let mut builder = Builder::new();
    builder.format(|buf, record| write!(buf, "{}: {}: {}\n", Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
                                        record.level(), record.args()))
           .filter_level(LevelFilter::Info);
    if env::var("LOG_LEVEL").is_ok() {
        builder = Builder::from_env("LOG_LEVEL");
    }

    builder.init();
}

/// Creates the given directory path if it doesn't exist already.
pub fn create_dir_if_not_exists<P>(path: P)
    where P: AsRef<Path>
{
    if !path.as_ref().is_dir() {
        info!("Creating {}.", path.as_ref().display());
        fs::create_dir_all(path).expect("cannot create directory");
    }
}

/// Removes any given path.
pub fn remove_any_path<P>(path: P)
    where P: AsRef<Path>
{
    if path.as_ref().is_dir() {
        fs::remove_dir_all(path).expect("cannot remove directory");
    } else {
        fs::remove_file(path).expect("cannot remove file");
    }
}

/// Read all bytes from the given file.
pub fn read_file<P>(path: P) -> io::Result<Vec<u8>>
    where P: AsRef<Path>
{
    let mut bytes = vec![];
    File::open(path.as_ref()).and_then(|mut fd| {
        fd.read_to_end(&mut bytes).map(|_| bytes)
    })
}
