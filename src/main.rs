//! Create a named pipe for cryptographically secure RNG (CRNG).

#![feature(test)]

mod core;
mod error;
mod rng;
mod stream;
mod workers;
use crate::error::Result;

extern crate libc;
extern crate test;

use clap::{arg, command, Command};
use std::path::Path;

fn create_argparser() -> Command<'static> {
    command!()
        .arg(
            arg!(-t --threads "The number of worker threads to spawn")
                .default_value("1")
                .validator(|s| s.parse::<usize>())
                .required(false),
        )
        .arg(
            arg!(-o --output "The file to write to; defaults to /dev/stdout")
                .default_value("/dev/stdout")
                .required(false),
        )
}

fn main() -> Result<()> {
    let matches = create_argparser().get_matches();

    let path = matches.value_of("output").expect("required");
    let path = Path::new(&path);
    let n_threads = matches.value_of_t("threads").expect("required");

    eprintln!("Writing stream to {}", path.display());
    stream::run_workers(path, n_threads)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::error::ErrorKind;
    use nix::{sys::stat::Mode, unistd};
    use std::error::Error;
    use std::fs::{self, File};
    use std::io::Read;
    use std::result::Result;
    use std::sync::atomic::Ordering;
    use tempfile::NamedTempFile;
    use test::Bencher;

    const BENCH_BUFSIZE: usize = 1 << 20;

    fn tmp_path() -> PathBuf {
        let tf = NamedTempFile::new().unwrap();
        let path = String::from(tf.path().to_str().unwrap());
        Path::new(&path).to_owned()
    }

    fn create_fifo(path: &Path, mode: Option<Mode>) -> Result<(), ErrorKind> {
        let mode = mode.unwrap_or(Mode::all());
        unistd::mkfifo(path, mode).expect("error creating FIFO pipe");
        Ok(())
    }

    fn bench_pipe_with_threads(b: &mut Bencher, n_threads: usize) -> Result<(), Box<dyn Error>> {
        let path = tmp_path();
        let path = path.as_path();
        create_fifo(&path, None)?;

        let pool = stream::start_workers(&path, n_threads)?;
        let mut file = File::open(&path)?;
        let mut buf = [0u8; BENCH_BUFSIZE];

        // Read from the pipe a few times to ensure that it's working correctly
        file.read_exact(&mut buf)?;
        file.read_exact(&mut buf)?;

        b.iter(|| file.read_exact(&mut buf));

        // Clean-up
        pool.running.store(false, Ordering::SeqCst);
        stream::join_workers(pool.handles)?;
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
