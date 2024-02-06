use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};
use uuid::Uuid;
use crate::p2p::config::CommsConfig;
use crate::p2p::messages::P2PMessage;
use crate::util::epoch_millis;

/// The P2PEnvelope contains a P2PMessage and additional meta-data.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct P2PEnvelope {
    /// The message
    pub message: P2PMessage,
    /// The peer from which the message was received.
    pub peer_id: Uuid,
    /// The connection id from which the message was received.
    pub connection_id: Uuid,
    /// The stream on which the message was received.
    pub stream_id: u16,
    /// The timestamp in milliseconds when the message was received.
    pub received_time: u64,
}

impl P2PEnvelope {
    pub fn new(message: P2PMessage, config: &CommsConfig) -> Self {
        P2PEnvelope {
            message,
            peer_id: config.peer_id.clone(),
            connection_id: config.connection_id.clone(),
            stream_id: config.stream_id,
            received_time: epoch_millis(),
        }
    }
}

/// Type alias for sender on the P2P Message channel
pub type P2PMessageChannelSender = Sender<Arc<P2PEnvelope>>;
/// Type alias for receiver on the P2P Message channel
pub type P2PMessageChannelReceiver = Receiver<Arc<P2PEnvelope>>;
