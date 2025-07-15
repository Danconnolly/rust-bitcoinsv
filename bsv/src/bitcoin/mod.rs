//! This module contains the bitcoin types and configuration for Bitcoin SV.

mod address;
mod base58ck;
mod block;
mod crypto;
mod encoding;
mod hash;
mod hash160;
mod header;
mod merkle;
mod params;
mod proptest_tests;
mod rules;
mod script;
mod stress_tests;
mod tx;
mod var_int;
mod var_int_edge_tests;

pub use self::address::Address;
pub use self::block::Block;
pub use self::crypto::{PrivateKey, PublicKey};
pub use self::encoding::Encodable;
pub use self::hash::Hash;
pub use self::header::{BlockHash, BlockHeader, MerkleRoot};
pub use self::merkle::{build_merkle_proof, calculate_merkle_root, verify_merkle_proof};
pub use self::params::{BlockchainId, KeyAddressKind};
pub use self::script::*;
pub use self::tx::{Outpoint, Tx, TxHash, TxInput, TxOutput};
pub use self::var_int::{
    varint_decode, varint_encode, varint_size, VARINT_MAX_SIZE, VARINT_MIN_SIZE,
};

pub use hex::{FromHex, ToHex};
