use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Bitcoin has multiple blockchains: "main", "test", "regtest", and "stn" chains.
///
/// In BitcoinSV we don't call these networks but blockchains. The P2P network is just a mechanism
/// for the applications to communicate, it does not define the blockchain. Its the other way around,
/// the blockchain defines the parameters used by the P2P network to communicate.
///
/// Use the From<&str> trait to translate string values, e.g. `let chain_id = BlockchainId::from("test");`
#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockchainId {
    #[serde(alias = "mainnet")]
    Main = 0,
    #[serde(alias = "testnet")]
    Test = 1,
    Stn = 2,
    Regtest = 3,
}

impl From<&str> for BlockchainId {
    fn from(value: &str) -> Self {
        match value {
            "main" => BlockchainId::Main,
            "mainnet" => BlockchainId::Main,
            "test" => BlockchainId::Test,
            "testnet" => BlockchainId::Test,
            "stn" => BlockchainId::Stn,
            "regtest" => BlockchainId::Regtest,
            _ => panic!("Unknown blockchain id: {}", value),
        }
    }
}

impl Display for BlockchainId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockchainId::Main => write!(f, "main"),
            BlockchainId::Test => write!(f, "test"),
            BlockchainId::Stn => write!(f, "stn"),
            BlockchainId::Regtest => write!(f, "regtest"),
        }
    }
}

/// KeyAddressKind enables us to differentiate whether a Key or Address is for the
/// production blockchain (mainnet) or whether it is for a test blockchain.
///
/// Unfortunately, the existing standard does not differentiate between different test blockchains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KeyAddressKind {
    Main = 0,
    NotMain = 1,
}

impl KeyAddressKind {
    /// The address prefix is used when encoding an Address.
    ///
    /// The prefix is prepended to the 160-byte hash of a public key before base-58 (with checksum)
    /// encoding the value to produce the Address. This is `base58Prefixes[PUBKEY_ADDRESS]`
    /// from the C reference code.
    pub fn get_address_prefix(&self) -> u8 {
        match self {
            KeyAddressKind::Main => 0x00,
            KeyAddressKind::NotMain => 0x6f,
        }
    }

    /// The private key prefix is used for the WIF encoding of a private key.
    ///
    /// This is `base58Prefixes[SECRET_KEY]` from the C reference code.
    pub fn get_private_key_prefix(&self) -> u8 {
        match self {
            KeyAddressKind::Main => 0x80,
            KeyAddressKind::NotMain => 0xef,
        }
    }
}

impl From<BlockchainId> for KeyAddressKind {
    fn from(value: BlockchainId) -> Self {
        match value {
            BlockchainId::Main => KeyAddressKind::Main,
            _ => KeyAddressKind::NotMain,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_serialize_blockchain() {
        let chain = BlockchainId::Main;
        let json = serde_json::to_string(&chain).unwrap();
        assert_eq!(json, "\"main\"");
        let chain = BlockchainId::Test;
        let json = serde_json::to_string(&chain).unwrap();
        assert_eq!(json, "\"test\"");
        let chain = BlockchainId::Stn;
        let json = serde_json::to_string(&chain).unwrap();
        assert_eq!(json, "\"stn\"");
        let chain = BlockchainId::Regtest;
        let json = serde_json::to_string(&chain).unwrap();
        assert_eq!(json, "\"regtest\"");
    }

    #[test]
    fn json_deserialize_blockchain() {
        let json = "\"main\"";
        let chain: BlockchainId = serde_json::from_str(json).unwrap();
        assert_eq!(chain, BlockchainId::Main);
        let chain = BlockchainId::from("test");
        assert_eq!(chain, BlockchainId::Test);
    }

    #[test]
    fn json_deserialize_old_names() {
        let json = "\"mainnet\"";
        let chain: BlockchainId = serde_json::from_str(json).unwrap();
        assert_eq!(chain, BlockchainId::Main);
        let json = "\"testnet\"";
        let chain: BlockchainId = serde_json::from_str(json).unwrap();
        assert_eq!(chain, BlockchainId::Test);
    }
}
