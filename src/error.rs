use std::convert::From;
use std::fmt;
use std::io;

/// A struct that encapsulates an error returned by a single
/// worker thread.
#[derive(Debug)]
pub struct WorkerError {
    thread_id: usize,
    error: ErrorKind,
}

impl WorkerError {
    pub fn new(thread_id: usize, error: ErrorKind) -> Self {
        WorkerError { thread_id, error }
    }
}

/// Custom `Error` kinds for `disco`.
#[derive(Debug)]
pub enum ErrorKind {
    GetRandomError(getrandom::Error),
    IOError(io::Error),
    WorkerErrors(Vec<WorkerError>),
}

impl std::error::Error for ErrorKind {}

/// Custom `Result` type for `disco`.
pub type Result<T> = core::result::Result<T, ErrorKind>;

impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "thread={}, err={}", self.thread_id, self.error)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::IOError(e) => {
                write!(f, "IOError: ")?;
                e.fmt(f)
            }
            ErrorKind::WorkerErrors(errs) => {
                write!(f, "(Worker errors)")?;
                for e in errs {
                    write!(f, " {}", e)?;
                }
                Ok(())
            }
            ErrorKind::GetRandomError(e) => {
                write!(f, "GetRandomError: {}", e)
            }
        }
    }
}

impl From<io::Error> for ErrorKind {
    fn from(e: io::Error) -> ErrorKind {
        ErrorKind::IOError(e)
    }
}
