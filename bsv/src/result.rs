use base58::FromBase58Error;
use bytes::TryGetError;
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
    /// Script is too large
    ScriptTooLarge,
    /// Script has too many operations
    ScriptTooManyOps,
    /// Script stack overflow
    ScriptStackOverflow,
    /// Script unbalanced conditional
    ScriptUnbalancedConditional,
    /// Script verify failed
    ScriptVerifyFailed,
    /// Script OP_RETURN encountered
    ScriptOpReturn,
    /// Script invalid stack operation
    ScriptInvalidStackOperation,
    /// Script number too large
    ScriptNumberTooLarge,
    /// Script disabled opcode
    ScriptDisabledOpcode,
    /// Script requires transaction context
    ScriptRequiresContext,
    /// Script reserved opcode
    ScriptReservedOpcode,
    /// Script unimplemented opcode
    ScriptUnimplementedOpcode,
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
    /// Error from TryGet
    TryGet(TryGetError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::BadArgument(s) => f.write_str(&format!("Bad argument: {}", s)),
            Error::BadData(s) => f.write_str(&format!("Bad data: {}", s)),
            Error::ChecksumMismatch => f.write_str("Checksum mismatch"),
            Error::WifTooLong => f.write_str("WIF too long"),
            Error::InvalidBlockchainSpecifier => f.write_str("Unknown blockchain"),
            Error::UnrecognizedOpCode => f.write_str("unrecognized opcode"),
            Error::DataTooSmall => f.write_str("data too small"),
            Error::DataTooLarge => f.write_str("data too large"),
            Error::ScriptTooLarge => f.write_str("script too large"),
            Error::ScriptTooManyOps => f.write_str("script has too many operations"),
            Error::ScriptStackOverflow => f.write_str("script stack overflow"),
            Error::ScriptUnbalancedConditional => f.write_str("script unbalanced conditional"),
            Error::ScriptVerifyFailed => f.write_str("script verify failed"),
            Error::ScriptOpReturn => f.write_str("script OP_RETURN encountered"),
            Error::ScriptInvalidStackOperation => f.write_str("script invalid stack operation"),
            Error::ScriptNumberTooLarge => f.write_str("script number too large"),
            Error::ScriptDisabledOpcode => f.write_str("script disabled opcode"),
            Error::ScriptRequiresContext => f.write_str("script requires transaction context"),
            Error::ScriptReservedOpcode => f.write_str("script reserved opcode"),
            Error::ScriptUnimplementedOpcode => f.write_str("script unimplemented opcode"),
            Error::Internal(s) => f.write_str(&format!("Internal error: {}", s)),
            Error::InternalError(e) => e.fmt(f),
            Error::FromHexError(e) => f.write_str(&format!("Hex decoding error: {}", e)),
            Error::FromBase58Error(e) => f.write_str(&format!("Base58 decoding error: {:?}", e)),
            Error::Secp256k1Error(e) => f.write_str(&format!("secpk256k1 error: {:?}", e)),
            Error::IOError(e) => f.write_str(&format!("IO error: {}", e)),
            Error::Utf8Error(e) => f.write_str(&format!("UTF8 error: {}", e)),
            Error::TryGet(e) => f.write_str(&format!("Tryget error: {}", e)),
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

impl From<TryGetError> for Error {
    fn from(e: TryGetError) -> Self {
        Error::TryGet(e)
    }
}

/// These are errors that are used internally within the library.
///
/// This is needed to enable Clone for minactor.
#[derive(Debug, Clone)]
pub enum InternalError {
    Dummy,
}

impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use InternalError::*;
        match self {
            Dummy => f.write_str("Dummy"),
        }
    }
}
