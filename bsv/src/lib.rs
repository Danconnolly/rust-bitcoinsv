//! Bitcoin SV library for Rust.
//!
//! This library is a work in progress. It is intended to provide a full featured library for using Bitcoin SV
//! in Rust applications at the infrastructure level. It is not intended to be a wallet or a client.

/// Contains functionality related to the core of Bitcoin SV. Transactions, Block Headers, etc.
pub mod bitcoin;

/// Contains functionality related to the Bitcoin SV peer-to-peer protocol and network.
pub mod p2p;

/// Contains useful utility functions.
pub mod util;

mod result;
pub use result::{BsvError, BsvResult};

// re-export the secp256k1 crate
pub extern crate secp256k1;
extern crate core;
