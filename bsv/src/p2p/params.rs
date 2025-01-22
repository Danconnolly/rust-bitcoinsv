use crate::bitcoin::BlockchainId;

/// Network Parameters for Bitcoin SV.
#[derive(Clone, Debug)]
pub struct NetworkParams {
    /// The magic bytes.
    pub magic: [u8; 4],
    /// The default port.
    pub port: u16,
}

impl From<BlockchainId> for NetworkParams {
    fn from(blockchain_id: BlockchainId) -> Self {
        match blockchain_id {
            BlockchainId::Main => NetworkParams {
                magic: [0xe3, 0xe1, 0xf3, 0xe8],
                port: 8333,
            },
            BlockchainId::Test => NetworkParams {
                magic: [0xf4, 0xe5, 0xf3, 0xf4],
                port: 18333,
            },
            BlockchainId::Regtest => NetworkParams {
                magic: [0xda, 0xb5, 0xbf, 0xfa],
                port: 18444,
            },
            BlockchainId::Stn => NetworkParams {
                magic: [0xfb, 0xce, 0xc4, 0xf9],
                port: 9333,
            },
        }
    }
}

/// Default max message payload size (32MB).
pub const DEFAULT_MAX_PAYLOAD_SIZE: u64 = 0x02000000;

/// Default max receive payload size (200MB).
// Initially, the maximum payload size is 32MB. Using protoconf, we specify to the peer that we can receive up to 200MB
// (configurable). If we receive a protoconf from the peer, then we can assume that our protoconf has been accepted.
// The protoconf from the peer will specify that maximum send message size. If we dont receive a protoconf from the peer,
// then we assume that the peer is using the default 32MB and we wont receive a larger message.
pub const DEFAULT_MAX_RECV_PAYLOAD_SIZE: u64 = 209_715_200;

/// Default excessive block size (10GB).
pub const DEFAULT_EXCESSIVE_BLOCK_SIZE: u64 = 10_000_000_000;

/// The maximum size of a transaction is 1GB, as described in the (Genesis Upgrade Specification)
/// [https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/genesis-spec.md#maximum-transaction-size].
pub const MAX_TX_SIZE: u64 = 1_000_000_000;

/// Protocol version supported by this library.
///
/// (P2P Large Message Support)[https://github.com/bitcoin-sv-specs/protocol/blob/master/p2p/large_messages.md] was added
/// to the BSV P2P Protocol on 2021-11-22. The protocol version was increased to 70016 with this change.
pub const PROTOCOL_VERSION: u32 = 70016;

/// Minimum protocol version supported by this library
pub const MIN_SUPPORTED_PROTOCOL_VERSION: u32 = 70015;
