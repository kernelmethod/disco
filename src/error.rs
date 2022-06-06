use nix::errno::Errno;
use std::fmt;
use std::io;

/// Custom `Error` kinds for `disco`.
#[derive(Debug)]
pub enum ErrorKind {
    UnixError(Errno),
    IOError(io::Error),
}

/// Custom `Result` type for `disco`.
pub type Result<T> = core::result::Result<T, ErrorKind>;

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::IOError(e) => {
                write!(f, "IOError: ")?;
                e.fmt(f)
            }
            ErrorKind::UnixError(e) => {
                write!(f, "UnixError: ")?;
                e.fmt(f)
            }
        }
    }
}
