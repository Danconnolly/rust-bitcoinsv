use crate::p2p::connection::ConnectionConfig;
use crate::p2p::params::{DEFAULT_MAX_PAYLOAD_SIZE, NetworkParams, PROTOCOL_VERSION};

/// CommsConfig is the active configuration for communicating with a peer.
///
/// It can be derived from the ConnectionConfig but is specific to a single stream and it has
/// additional configuration options that are determined during operation.
///
/// These configuration options are used throughout the P2P protocol to determine message
/// limits and other communication parameters.
#[derive(Debug, Clone)]
pub struct CommsConfig {
    /// The magic bytes used in the message header.
    pub magic: [u8; 4],
    /// The maximum payload size we want to receive, using protoconf.
    pub max_recv_payload_size: u64,
    /// The maximum payload size the peer wants to receive.
    pub max_send_payload_size: u64,
    /// The maximum size of a block that we will accept.
    pub excessive_block_size: u64,
    /// The protocol version used by the remote peer.
    pub protocol_version: u32,
}

impl CommsConfig {
    pub fn new(config: &ConnectionConfig) -> CommsConfig {
        let np = NetworkParams::from(config.blockchain);
        CommsConfig {
            magic: np.magic.clone(),
            max_recv_payload_size: config.max_recv_payload_size,
            max_send_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,
            excessive_block_size: config.excessive_block_size,
            protocol_version: PROTOCOL_VERSION,
        }
    }
}
