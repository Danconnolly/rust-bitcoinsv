//! Bitcoin SV library for Rust.
//!
//! This library is a work in progress. It is intended to provide a full featured library for using Bitcoin SV
//! in Rust applications at the infrastructure level. It is not intended to be a wallet or a client.
//!
//! This library is opinionated, in the sense that it does not stick to convention; the library presents
//! a view of Bitcoin that is possibly different from the norm. The library is not a translation of some
//! other more established library, it is a re-write from ground level principles.
//!
//! * the library defines [BlockchainId] to distinguish between "mainnet", "testnet",
//! etc, not "networks". The key feature that distinguishes these blockchains is the genesis block
//! not the network. The P2P network is just a means for software to communicate, it does not define
//! the blockchain.
//!
//! * the library will probably never support old versions of Bitcoin. Support for old versions is dead
//! code and will be removed as quickly as possible.
//! 
//! [BlockchainId]: crate::bitcoin::BlockchainId

/// Functionality related to the core of Bitcoin SV. Transactions, Block Headers, etc.
pub mod bitcoin;

#[cfg(feature = "dev_p2p")]
/// Functionality related to the Bitcoin SV peer-to-peer protocol and network.
pub mod p2p;

/// Utility functions.
pub mod util;

mod result;
pub use result::{Error, Result};
