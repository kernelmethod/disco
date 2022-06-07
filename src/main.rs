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
use std::path::Path;

fn create_argparser() -> Command<'static> {
    command!()
        .arg(arg!([path] "The path to the named pipe that should be created").required(true))
        .arg(
            arg!(-t --threads "The number of worker threads to spawn")
                .default_value("1")
                .validator(|s| s.parse::<usize>())
                .required(false),
        )
}

fn main() -> Result<()> {
    let matches = create_argparser().get_matches();
    let path = matches.value_of("path").expect("required");
    let path = Path::new(&path);
    let display = path.display();
    let n_threads = matches.value_of_t("threads").expect("required");

    // Attempt to create the named pipe
    let mode = Mode::all();
    match unistd::mkfifo(path, mode) {
        Err(e) => return Err(ErrorKind::UnixError(e)),
        _ => {}
    }

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
