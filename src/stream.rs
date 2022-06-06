use crate::core::*;
use crate::error::{ErrorKind, Result, WorkerError};
use crate::rng::CRNG;

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::{fs, io};

#[derive(Debug)]
pub struct WorkerSpec {
    pathbuf: PathBuf,
    running: Arc<AtomicBool>,
}

impl WorkerSpec {
    pub fn new(path: &Path, running: &Arc<AtomicBool>) -> Self {
        WorkerSpec {
            pathbuf: path.clone().to_owned(),
            running: running.clone(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn path(&self) -> &Path {
        self.pathbuf.as_path()
    }
}

fn open_pipe(path: &Path) -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .read(false)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&path)
}

fn run_worker(spec: WorkerSpec) -> Result<()> {
    let mut rng = CRNG::new();

    println!(
        "Started worker {}",
        thread::current().name().unwrap_or("???"),
    );

    while spec.is_running() {
        let mut file = match open_pipe(&spec.path()) {
            Err(e) => {
                if Some(libc::ENXIO) == e.raw_os_error() {
                    // No clients have opened the pipe yet
                    thread::sleep(default_sleep_time());
                    continue;
                } else {
                    return Err(ErrorKind::IOError(e));
                }
            }
            Ok(file) => file,
        };

        // Repeatedly write blocks of random data to the named pipe
        while spec.is_running() {
            match file.write_all(rng.regenerate()) {
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
                Ok(_) => {}
            };
        }
    }

    // Perform rng.regenerate() one more time to erase the final
    // key state
    rng.regenerate();

    Ok(())
}

pub fn start_workers(path: &Path, n_workers: usize) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));

    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let errors = (0..n_workers)
        .map(|i| {
            let spec = WorkerSpec::new(&path, &running);
            thread::Builder::new()
                .name(format!("worker {}", i))
                .spawn(move || run_worker(spec))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .enumerate()
        .filter_map(|(i, h)| {
            let h = match h {
                Err(e) => {
                    let err = ErrorKind::IOError(e);
                    return Some(WorkerError::new(i, err));
                }
                Ok(h) => h,
            };

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

    if errors.len() > 0 {
        Err(ErrorKind::WorkerErrors(errors))
    } else {
        Ok(())
    }
}