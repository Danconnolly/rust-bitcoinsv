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

    // P2P module errors
    /// Peer store error
    PeerStoreError(String),
    /// Peer not found
    PeerNotFound(uuid::Uuid),
    /// Duplicate peer
    DuplicatePeer,
    /// Connection refused (retryable)
    ConnectionRefused,
    /// Connection timeout (retryable)
    ConnectionTimeout,
    /// Connection reset (retryable)
    ConnectionReset,
    /// Connection failed (non-retryable)
    ConnectionFailed(String),
    /// Handshake timeout (non-retryable)
    HandshakeTimeout,
    /// Handshake failed (non-retryable)
    HandshakeFailed(String),
    /// Network mismatch during handshake
    NetworkMismatch { expected: String, received: String },
    /// Blockchain mismatch during handshake
    BlockchainMismatch { received: String },
    /// Banned user agent
    BannedUserAgent { user_agent: String },
    /// Invalid configuration
    InvalidConfiguration(String),
    /// Invalid connection limits
    InvalidConnectionLimits { target: usize, max: usize },
    /// DNS resolution failed
    DnsResolutionFailed(String),
    /// Channel send error
    ChannelSendError,
    /// Channel receive error
    ChannelReceiveError,
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
            // P2P errors
            Error::PeerStoreError(s) => f.write_str(&format!("Peer store error: {}", s)),
            Error::PeerNotFound(id) => f.write_str(&format!("Peer not found: {}", id)),
            Error::DuplicatePeer => f.write_str("Duplicate peer"),
            Error::ConnectionRefused => f.write_str("Connection refused"),
            Error::ConnectionTimeout => f.write_str("Connection timeout"),
            Error::ConnectionReset => f.write_str("Connection reset"),
            Error::ConnectionFailed(s) => f.write_str(&format!("Connection failed: {}", s)),
            Error::HandshakeTimeout => f.write_str("Handshake timeout"),
            Error::HandshakeFailed(s) => f.write_str(&format!("Handshake failed: {}", s)),
            Error::NetworkMismatch { expected, received } => f.write_str(&format!(
                "Network mismatch: expected {}, got {}",
                expected, received
            )),
            Error::BlockchainMismatch { received } => {
                f.write_str(&format!("Blockchain mismatch: {}", received))
            }
            Error::BannedUserAgent { user_agent } => {
                f.write_str(&format!("Banned user agent: {}", user_agent))
            }
            Error::InvalidConfiguration(s) => f.write_str(&format!("Invalid configuration: {}", s)),
            Error::InvalidConnectionLimits { target, max } => f.write_str(&format!(
                "Invalid connection limits: target={}, max={}",
                target, max
            )),
            Error::DnsResolutionFailed(s) => f.write_str(&format!("DNS resolution failed: {}", s)),
            Error::ChannelSendError => f.write_str("Channel send error"),
            Error::ChannelReceiveError => f.write_str("Channel receive error"),
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
