//! Create a named pipe for cryptographically secure RNG (CRNG).

#![feature(test)]

mod core;
mod error;
mod rng;
mod stream;
mod workers;
use crate::error::{ErrorKind, Result};

extern crate libc;
extern crate test;

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
    println!("Created FIFO at {}", display);
    let ret = stream::run_workers(&path, n_threads);

    println!("Closing stream...");

    match fs::remove_file(&path) {
        Err(e) => {
            eprintln!("Unable to remove {}: {}", display, e);
        }
        _ => {}
    };

    ret
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fs::{self, File};
    use std::io::Read;
    use std::result::Result;
    use std::sync::atomic::Ordering;
    use test::Bencher;

    const BENCH_BUFSIZE: usize = 1 << 20;

    fn bench_pipe_with_threads(b: &mut Bencher, n_threads: usize) -> Result<(), Box<dyn Error>> {
        let path = get_fifo_path(None);
        create_fifo(&path, None)?;

        let (running, handles) = stream::start_workers(&path, n_threads)?;
        let mut file = File::open(&path)?;
        let mut buf = [0u8; BENCH_BUFSIZE];

        // Read from the pipe a few times to ensure that it's working correctly
        file.read_exact(&mut buf)?;
        file.read_exact(&mut buf)?;

        b.iter(|| file.read_exact(&mut buf));

        // Clean-up
        running.store(false, Ordering::SeqCst);
        stream::join_workers(handles)?;
        fs::remove_file(&path)?;
        Ok(())
    }

    #[bench]
    fn bench_pipe_n1(b: &mut Bencher) -> Result<(), Box<dyn Error>> {
        bench_pipe_with_threads(b, 1)
    }

    #[bench]
    fn bench_pipe_n2(b: &mut Bencher) -> Result<(), Box<dyn Error>> {
        bench_pipe_with_threads(b, 2)
    }

    #[bench]
    fn bench_pipe_n4(b: &mut Bencher) -> Result<(), Box<dyn Error>> {
        bench_pipe_with_threads(b, 4)
    }

    #[bench]
    fn bench_pipe_n8(b: &mut Bencher) -> Result<(), Box<dyn Error>> {
        bench_pipe_with_threads(b, 8)
    }

    #[cfg(target_os = "linux")]
    #[bench]
    fn bench_urandom(b: &mut Bencher) -> std::result::Result<(), Box<dyn Error>> {
        let mut file = File::open("/dev/urandom")?;
        let mut buf = [0u8; BENCH_BUFSIZE];
        b.iter(|| file.read_exact(&mut buf));

        Ok(())
    }
}
