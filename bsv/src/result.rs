use std::io;
use hex::FromHexError;

// Standard error & result types.
// Code based on `<https://github.com/brentongunning/rust-sv>`


/// Standard Result used in the library
pub type Result<T> = std::result::Result<T, Error>;

/// Standard error type used in the library
#[derive(Debug)]
pub enum Error {
    /// An argument provided is invalid
    BadArgument(String),
    /// Hex string could not be decoded
    FromHexError(FromHexError),
    /// Standard library IO error
    IOError(io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::BadArgument(s) => f.write_str(&format!("Bad argument: {}", s)),
            Error::FromHexError(e) => f.write_str(&format!("Hex decoding error: {}", e)),
            Error::IOError(e) => f.write_str(&format!("IO error: {}", e)),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::BadArgument(_) => "Bad argument",
            Error::FromHexError(_) => "Hex decoding error",
            Error::IOError(_) => "IO error",
        }
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            Error::FromHexError(e) => Some(e),
            Error::IOError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<FromHexError> for Error {
    fn from(e: FromHexError) -> Self {
        Error::FromHexError(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IOError(e)
    }
}
