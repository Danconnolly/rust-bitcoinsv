use uuid::Uuid;
use crate::p2p::connection::ConnectionConfig;
use crate::p2p::params::{DEFAULT_MAX_PAYLOAD_SIZE, NetworkParams, PROTOCOL_VERSION};

// todo: rename this
/// CommsConfig is the context for the communication across a single stream.
///
/// These parameters are used throughout the P2P protocol to determine message
/// limits and other communication patterns.
///
/// It can be derived from the ConnectionConfig but is specific to a single stream. Most of the parameters
/// are static and do not change during the lifetime of the stream, but there are a couple that are determined
/// during the extended handshake and will need to be updated.
///
/// It is expected that this struct will be a single instance that is potentially shared by several threads (for
/// example a reader and writer thread).
///
/// At the moment this is used by obtaining a clone using a read lock before every read and write but this is
/// inefficient and should be changed to a more efficient method.
#[derive(Debug, Clone)]
pub struct CommsConfig {
    /// The identifier of the peer being connected to.
    pub peer_id: Uuid,
    /// The identifier of the connection.
    pub connection_id: Uuid,
    /// The identifier of the stream.
    pub stream_id: u16,
    /// Send control messages to data channel?
    pub send_control_messages: bool,
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
    pub fn new(config: &ConnectionConfig, peer_id: &Uuid) -> CommsConfig {
        let np = NetworkParams::from(config.blockchain);
        CommsConfig {
            peer_id: peer_id.clone(), connection_id: Uuid::new_v4(), stream_id: 0,
            send_control_messages: config.send_control_messages, magic: np.magic.clone(),
            max_recv_payload_size: config.max_recv_payload_size,
            max_send_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,
            excessive_block_size: config.excessive_block_size,
            protocol_version: PROTOCOL_VERSION,
        }
    }
}

impl Default for CommsConfig {
    fn default() -> Self {
        let connection_config = ConnectionConfig::default();
        CommsConfig::new(&connection_config, &Uuid::new_v4())
    }
}
