//! Bitcoin P2P network protocol implementation
//!
//! This module implements the Bitcoin peer-to-peer network protocol,
//! including message serialization, deserialization, and handling.

mod message;
mod protocol;

pub use self::message::*;
pub use self::protocol::*;
