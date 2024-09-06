/// There are four blockchains: mainnet, testnet, stn, and regtest.

use serde::{Deserialize, Serialize};

/// Bitcoin has multiple blockchains: "main", "test", "regtest", and "stn" chains.
///
/// In BitcoinSV we don't call these networks but blockchains. The P2P network is just a mechanism
/// for the applications to communicate, it does not define the blockchain. Its the other way around,
/// the blockchain defines the parameters used by the P2P network to communicate.
#[derive(Copy, Clone, PartialEq, Debug)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockchainId {
    #[serde(alias = "mainnet")]
    Main = 0,
    #[serde(alias = "testnet")]
    Test = 1,
    Stn = 2,
    Regtest = 3,
}

/// Each blockchain has some different parameters.
pub struct BlockchainParams {
    /// A byte that is prefixed to a private key when it is exported.
    pub private_key_prefix: u8,
}

impl BlockchainParams {
    /// Get the BlockchainParams for a specific blockchain.
    pub fn get_params(blockchain: BlockchainId) -> Self {
        match blockchain {
            BlockchainId::Main => BlockchainParams {
                private_key_prefix: 0x80,
            },
            BlockchainId::Test => BlockchainParams {
                private_key_prefix: 0xef,
            },
            BlockchainId::Regtest => BlockchainParams {
                private_key_prefix: 0xef,
            },
            BlockchainId::Stn => BlockchainParams {
                private_key_prefix: 0xef,
            },
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