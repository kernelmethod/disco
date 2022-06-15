use crate::core::*;
use crate::error::{ErrorKind, Result, WorkerError};
use crate::rng::CryptoRng;
use crate::workers::{WorkerPool, WorkerSpec};

use std::fs;
use std::io::{self, BufWriter, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::panic;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};

fn open(path: &Path) -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .read(false)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&path)
}

fn run_worker(spec: WorkerSpec) -> Result<()> {
    let mut rng = CryptoRng::from_entropy()?;

    while spec.is_running() {
        let mut stream = match open(spec.path()) {
            Err(e) => {
                if Some(libc::ENXIO) == e.raw_os_error() {
                    // No clients have opened the pipe yet
                    thread::sleep(default_sleep_time());
                    continue;
                } else {
                    return Err(ErrorKind::IOError(e));
                }
            }
            Ok(file) => BufWriter::new(file),
        };

        // Repeatedly write blocks of random data to the named pipe
        while spec.is_running() {
            if let Err(e) = stream.write_all(rng.regenerate()) {
                match e.kind() {
                    // Pipe was closed by client
                    io::ErrorKind::BrokenPipe => {
                        break;
                    }
                    io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    // Other error
                    _ => return Err(ErrorKind::IOError(e)),
                }
            };
        }
    }

    // Perform rng.regenerate() one more time to erase the final
    // key state
    rng.regenerate();

    Ok(())
}

pub fn run_workers(path: &Path, n_workers: usize) -> Result<()> {
    let pool = start_workers(path, n_workers)?;
    let r = pool.running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    join_workers(pool.handles)
}

pub fn join_workers(handles: Vec<JoinHandle<Result<()>>>) -> Result<()> {
    let errors = handles
        .into_iter()
        .enumerate()
        .filter_map(|(i, h)| {
            match h.join() {
                Ok(res) => match res {
                    Err(e) => Some(WorkerError::new(i, e)),
                    _ => None,
                },
                // In theory we should only reach this point if one of the
                // threads panics
                Err(e) => panic::resume_unwind(e),
            }
        })
        .collect::<Vec<_>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ErrorKind::WorkerErrors(errors))
    }
}

pub fn start_workers(path: &Path, n_workers: usize) -> Result<WorkerPool> {
    let running = Arc::new(AtomicBool::new(true));

    let handles = (0..n_workers)
        .map(|i| {
            let spec = WorkerSpec::new(path, &running);
            thread::Builder::new()
                .name(format!("worker {}", i))
                .spawn(move || run_worker(spec))
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(WorkerPool { running, handles })
}
