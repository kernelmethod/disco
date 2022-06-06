//! Create a named pipe for cryptographically secure RNG (CRNG).

#![feature(io_error_uncategorized)]
#![feature(scoped_threads)]

mod core;
mod error;
mod stream;
use crate::error::{ErrorKind, Result};

extern crate libc;

use clap::Parser;
use nix::{sys::stat::Mode, unistd};
use std::fs;
use std::path::Path;

/// Arguments that can be passed in to the program.
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The path to the named pipe that should be created.
    path: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let path = Path::new(&args.path);
    let display = path.display();

    // Attempt to create the named pipe
    let mode = Mode::S_IRWXU | Mode::S_IRGRP | Mode::S_IROTH;
    match unistd::mkfifo(path, mode) {
        Err(e) => return Err(ErrorKind::UnixError(e)),
        _ => {}
    }

    let ret = stream::start_workers(&path, 3);

    println!("Closing stream...");

    match fs::remove_file(&path) {
        Err(e) => {
            eprintln!("Unable to remove {}: {}", display, e);
        }
        _ => {}
    };

    ret
}
