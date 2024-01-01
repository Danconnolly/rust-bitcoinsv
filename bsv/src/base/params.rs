/// There are four blockchains: mainnet, testnet, stn, and regtest.

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Debug)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Blockchain {
    #[serde(alias = "main")]
    Mainnet = 0,
    Testnet = 1,
    Stn = 2,
    Regtest = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_serialize_blockchain() {
        let chain = Blockchain::Mainnet;
        let json = serde_json::to_string(&chain).unwrap();
        assert_eq!(json, "\"mainnet\"");
    }

    #[test]
    fn json_deserialize_blockchain() {
        let json = "\"mainnet\"";
        let chain: Blockchain = serde_json::from_str(json).unwrap();
        assert_eq!(chain, Blockchain::Mainnet);
    }
}