use std::io;
use std::string::FromUtf8Error;
use base58::FromBase58Error;
use hex::FromHexError;

// Standard error & result types.


/// Standard Result used in the library
pub type BsvResult<T> = Result<T, BsvError>;

/// Standard error type used in the library
#[derive(Debug)]
pub enum BsvError {
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

impl std::fmt::Display for BsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BsvError::BadArgument(s) => f.write_str(&format!("Bad argument: {}", s)),
            BsvError::BadData(s) => f.write_str(&format!("Bad data: {}", s)),
            BsvError::ChecksumMismatch => f.write_str(&"Checksum mismatch".to_string()),
            BsvError::Internal(s) => f.write_str(&format!("Internal error: {}", s)), // Added this line
            BsvError::FromHexError(e) => f.write_str(&format!("Hex decoding error: {}", e)),
            BsvError::FromBase58Error(e) => f.write_str(&format!("Base58 decoding error: {:?}", e)),
            BsvError::IOError(e) => f.write_str(&format!("IO error: {}", e)),
            BsvError::Utf8Error(e) => f.write_str(&format!("UTF8 error: {}", e)),
        }
    }
}

impl std::error::Error for BsvError {
    fn description(&self) -> &str {
        match self {
            BsvError::BadArgument(_) => "Bad argument",
            BsvError::BadData(_) => "Bad data",
            BsvError::ChecksumMismatch => "Checksum mismatch",
            BsvError::Internal(_) => "Internal error", // Added this line
            BsvError::FromHexError(_) => "Hex decoding error",
            BsvError::FromBase58Error(_) => "Base58 decoding error",
            BsvError::IOError(_) => "IO error",
            BsvError::Utf8Error(_) => "UTF8 error",
        }
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            BsvError::FromHexError(e) => Some(e),
            BsvError::IOError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<FromHexError> for BsvError {
    fn from(e: FromHexError) -> Self {
        BsvError::FromHexError(e)
    }
}

impl From<FromBase58Error> for BsvError {
    fn from(e: FromBase58Error) -> Self {
        BsvError::FromBase58Error(e)
    }
}

impl From<io::Error> for BsvError {
    fn from(e: io::Error) -> Self {
        BsvError::IOError(e)
    }
}

impl From<FromUtf8Error> for BsvError {
    fn from(e: FromUtf8Error) -> Self {
        BsvError::Utf8Error(e)
    }
}
