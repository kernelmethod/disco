//! Create a named pipe for cryptographically secure RNG (CRNG).

#![feature(io_error_uncategorized)]

mod error;
use crate::error::{ErrorKind, Result};

extern crate libc;

use clap::Parser;
use nix::{errno, sys::stat::Mode, unistd};
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::{thread, time};

const ENXIO: i32 = errno::Errno::ENXIO as i32;
const fn sleep_time() -> time::Duration {
    time::Duration::from_millis(20)
}

/// Arguments that can be passed in to the program.
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The path to the named pipe that should be created.
    path: String,
}

fn open_pipe(path: &Path) -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .read(false)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&path)
}

fn write_stream(path: &Path) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let mut rng = ChaCha20Rng::from_entropy();
    let mut buffer = [0u8; 64];
    rng.fill(&mut buffer);

    while running.load(Ordering::SeqCst) {
        let mut file = match open_pipe(&path) {
            Err(e) => match e.raw_os_error() {
                // No clients have opened the pipe yet
                Some(ENXIO) => {
                    thread::sleep(sleep_time());
                    continue;
                }
                // Other error
                _ => return Err(ErrorKind::IOError(e)),
            },
            Ok(file) => file,
        };

        // Repeatedly write blocks of random data to the named pipe
        while running.load(Ordering::SeqCst) {
            match file.write_all(&buffer) {
                Err(e) => match e.kind() {
                    // Pipe was closed by client
                    io::ErrorKind::BrokenPipe => {
                        break;
                    }
                    io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    // Other error
                    _ => return Err(ErrorKind::IOError(e)),
                },
                Ok(_) => {
                    rng.fill(&mut buffer);
                }
            };
        }
    }

    Ok(())
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

    let result = write_stream(&path);

    println!("Closing stream...");

    match fs::remove_file(&path) {
        Err(e) => {
            eprintln!("Unable to remove {}: {}", display, e);
        }
        _ => {}
    };

    result
}
