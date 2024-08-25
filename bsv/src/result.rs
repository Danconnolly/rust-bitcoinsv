use std::io;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use base58::FromBase58Error;
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
    /// The data provided is invalid
    BadData(String),
    /// The data did not match the checksum.
    ChecksumMismatch,
    /// Internal error
    Internal(String),
    /// Hex string could not be decoded
    FromHexError(FromHexError),
    /// Base58 string could not be decoded
    FromBase58Error(FromBase58Error),
    /// Standard library IO error
    IOError(io::Error),
    /// String conversion error
    Utf8Error(FromUtf8Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::BadArgument(s) => f.write_str(&format!("Bad argument: {}", s)),
            Error::BadData(s) => f.write_str(&format!("Bad data: {}", s)),
            Error::ChecksumMismatch => f.write_str(&"Checksum mismatch".to_string()),
            Error::Internal(s) => f.write_str(&format!("Internal error: {}", s)), // Added this line
            Error::FromHexError(e) => f.write_str(&format!("Hex decoding error: {}", e)),
            Error::FromBase58Error(e) => f.write_str(&format!("Base58 decoding error: {:?}", e)),
            Error::IOError(e) => f.write_str(&format!("IO error: {}", e)),
            Error::Utf8Error(e) => f.write_str(&format!("UTF8 error: {}", e)),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::BadArgument(_) => "Bad argument",
            Error::BadData(_) => "Bad data",
            Error::ChecksumMismatch => "Checksum mismatch",
            Error::Internal(_) => "Internal error", // Added this line
            Error::FromHexError(_) => "Hex decoding error",
            Error::FromBase58Error(_) => "Base58 decoding error",
            Error::IOError(_) => "IO error",
            Error::Utf8Error(_) => "UTF8 error",
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

impl From<FromBase58Error> for Error {
    fn from(e: FromBase58Error) -> Self {
        Error::FromBase58Error(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IOError(e)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Self {
        Error::Utf8Error(e)
    }
}
