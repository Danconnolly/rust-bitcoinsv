use base58::FromBase58Error;
use hex::FromHexError;
use std::fmt::Formatter;
use std::io;
use std::string::FromUtf8Error;

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
    /// The WIF provided was too long.
    WifTooLong,
    /// The blockchain specifier was not recognized.
    InvalidBlockchainSpecifier,
    /// Unrecognized Opcode
    UnrecognizedOpCode,
    /// The data provided is too small to perform the operation.
    DataTooSmall,
    /// The data provided is too large to perform the operation.
    DataTooLarge,
    /// Internal error
    Internal(String),
    /// Internal errors
    InternalError(InternalError),
    /// Hex string could not be decoded
    FromHexError(FromHexError),
    /// Base58 string could not be decoded
    FromBase58Error(FromBase58Error),
    /// secp256k1 library error
    Secp256k1Error(secp256k1::Error),
    /// Standard library IO error
    IOError(io::Error),
    /// String conversion error
    Utf8Error(FromUtf8Error),
    /// Error from minactor
    MinActorError(minactor::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::BadArgument(s) => f.write_str(&format!("Bad argument: {}", s)),
            Error::BadData(s) => f.write_str(&format!("Bad data: {}", s)),
            Error::ChecksumMismatch => f.write_str(&"Checksum mismatch".to_string()),
            Error::WifTooLong => f.write_str(&"WIF too long".to_string()),
            Error::InvalidBlockchainSpecifier => f.write_str(&"Unknown blockchain".to_string()),
            Error::UnrecognizedOpCode => f.write_str(&"unrecognized opcode".to_string()),
            Error::DataTooSmall => f.write_str(&"data too small".to_string()),
            Error::DataTooLarge => f.write_str(&"data too large".to_string()),
            Error::Internal(s) => f.write_str(&format!("Internal error: {}", s)), // Added this line
            Error::InternalError(e) => e.fmt(f),
            Error::FromHexError(e) => f.write_str(&format!("Hex decoding error: {}", e)),
            Error::FromBase58Error(e) => f.write_str(&format!("Base58 decoding error: {:?}", e)),
            Error::Secp256k1Error(e) => f.write_str(&format!("secpk256k1 error: {:?}", e)),
            Error::IOError(e) => f.write_str(&format!("IO error: {}", e)),
            Error::Utf8Error(e) => f.write_str(&format!("UTF8 error: {}", e)),
            Error::MinActorError(e) => f.write_str(&format!("Minactor error: {:?}", e)), // todo: revert to display when implemented
        }
    }
}

impl From<InternalError> for Error {
    fn from(value: InternalError) -> Self {
        Error::InternalError(value)
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

impl From<secp256k1::Error> for Error {
    fn from(e: secp256k1::Error) -> Self {
        Error::Secp256k1Error(e)
    }
}

impl From<minactor::Error> for Error {
    fn from(e: minactor::Error) -> Self {
        Error::MinActorError(e)
    }
}

/// These are errors that are used internally within the library.
///
/// This is needed to enable Clone for minactor.
#[derive(Debug, Clone)]
pub(crate) enum InternalError {
    Dummy,
}

impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use InternalError::*;
        match self {
            Dummy => f.write_str(&"Dummy".to_string()),
        }
    }
}
