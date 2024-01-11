use crate::bitcoin::BlockchainId;

/// Network Parameters for Bitcoin SV.
#[derive(Clone, Debug)]
pub struct NetworkParams {
    /// The magic bytes.
    pub magic: [u8; 4],
    /// default port.
    pub port: u16,
}

impl From<BlockchainId> for NetworkParams {
    fn from(blockchain_id: BlockchainId) -> Self {
        match blockchain_id {
            BlockchainId::Mainnet => {
              NetworkParams {
                  magic:[0xe3, 0xe1, 0xf3, 0xe8],
                  port: 8333,
              }
            },
            BlockchainId::Testnet => {
                NetworkParams {
                    magic: [0xf4, 0xe5, 0xf3, 0xf4],
                    port: 18333,
                }
            },
            BlockchainId::Regtest => {
                NetworkParams {
                    magic: [0xda, 0xb5, 0xbf, 0xfa],
                    port: 18444,
                }
            },
            BlockchainId::Stn => {
                NetworkParams {
                    magic: [0xfb, 0xce, 0xc4, 0xf9],
                    port: 9333,
                }
            },
        }
    }
}