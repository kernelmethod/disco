use crate::core::*;
use crate::error::{ErrorKind, Result, WorkerError};
use crate::rng::CRNG;

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};

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
    let mut rng = CRNG::from_entropy()?;

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

pub fn run_workers(path: &Path, n_workers: usize) -> Result<()> {
    let (running, handles) = start_workers(path, n_workers);
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    join_workers(handles)
}

pub fn join_workers(
    handles: Vec<std::result::Result<JoinHandle<Result<()>>, WorkerError>>,
) -> Result<()> {
    let errors = handles
        .into_iter()
        .enumerate()
        .filter_map(|(i, h)| {
            let h = match h {
                Ok(h) => h,
                Err(e) => return Some(e),
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

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ErrorKind::WorkerErrors(errors))
    }
}

pub fn start_workers(
    path: &Path,
    n_workers: usize,
) -> (
    Arc<AtomicBool>,
    Vec<std::result::Result<JoinHandle<Result<()>>, WorkerError>>,
) {
    let running = Arc::new(AtomicBool::new(true));

    let handles = (0..n_workers)
        .map(|i| {
            let spec = WorkerSpec::new(&path, &running);
            thread::Builder::new()
                .name(format!("worker {}", i))
                .spawn(move || run_worker(spec))
        })
        .enumerate()
        .map(|(i, h)| match h {
            Err(e) => {
                let err = ErrorKind::IOError(e);
                Err(WorkerError::new(i, err))
            }
            Ok(h) => Ok(h),
        })
        .collect::<Vec<_>>();

    (running, handles)
}
