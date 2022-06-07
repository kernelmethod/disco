//! Create a named pipe for cryptographically secure RNG (CRNG).

mod core;
mod error;
mod rng;
mod stream;
use crate::error::{ErrorKind, Result};

extern crate libc;

use clap::{arg, command, Command};
use nix::{sys::stat::Mode, unistd};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "tempfile")]
use tempfile::NamedTempFile;

fn create_argparser() -> Command<'static> {
    let cmd = command!().arg(
        arg!(-t --threads "The number of worker threads to spawn")
            .default_value("1")
            .validator(|s| s.parse::<usize>())
            .required(false),
    );

    if cfg!(feature = "tempfile") {
        let help = concat!(
            "The path to the named pipe that should be created. ",
            "If unset, a temporary file will be created for the pipe."
        );

        cmd.arg(arg!([path]).help(help).required(false))
    } else {
        cmd.arg(arg!([path] "The path to the named pipe that should be created.").required(true))
    }
}

fn get_fifo_path(path: Option<&str>) -> PathBuf {
    let path = if cfg!(feature = "tempfile") {
        match path {
            Some(path) => String::from(path),
            None => {
                let tf = NamedTempFile::new().unwrap();
                let path = String::from(tf.path().to_str().unwrap());
                println!("Creating FIFO at {}", path);
                path
            }
        }
    } else {
        String::from(path.expect("required"))
    };

    Path::new(&path).to_owned()
}

fn create_fifo(path: &Path, mode: Option<Mode>) -> Result<()> {
    let mode = mode.unwrap_or(Mode::all());
    match unistd::mkfifo(path, mode) {
        Err(e) => Err(ErrorKind::UnixError(e)),
        _ => Ok(()),
    }
}

fn main() -> Result<()> {
    let matches = create_argparser().get_matches();
    let path = get_fifo_path(matches.value_of("path"));
    let path = path.as_path();
    let display = path.display();
    let n_threads = matches.value_of_t("threads").expect("required");

    // Attempt to create the named pipe
    create_fifo(&path, None)?;
    let ret = stream::start_workers(&path, n_threads);

    println!("Closing stream...");

    match fs::remove_file(&path) {
        Err(e) => {
            eprintln!("Unable to remove {}: {}", display, e);
        }
        _ => {}
    };

    ret
}
