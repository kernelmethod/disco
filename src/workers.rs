use crate::error::Result;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct WorkerPool {
    pub running: Arc<AtomicBool>,
    pub handles: Vec<JoinHandle<Result<()>>>,
}

/// Struct specifying the parameters and work that a given
/// thread will perform.
#[derive(Debug)]
pub struct WorkerSpec {
    /// Path to the named pipe.
    pathbuf: PathBuf,

    /// `AtomicBool` that can be checked to determine whether
    /// or not the worker should continue working.
    running: Arc<AtomicBool>,
}

impl WorkerSpec {
    /// Create a new `WorkerSpec` instance.
    pub fn new(path: &Path, running: &Arc<AtomicBool>) -> Self {
        WorkerSpec {
            pathbuf: path.to_owned(),
            running: running.clone(),
        }
    }

    /// Check whether the worker is still supposed to be running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Return the path to the FIFO pipe as an instance of `&Path`.
    pub fn path(&self) -> &Path {
        self.pathbuf.as_path()
    }
}
