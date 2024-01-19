use tokio::sync::broadcast::{Receiver, Sender};

mod node_addr;
mod version;
mod messages;
mod msg_header;
mod protoconf;

// the individual P2P messages
pub use node_addr::NodeAddr;
pub use protoconf::Protoconf;
pub use version::Version;

// P2P message
pub use messages::{P2PMessage, P2PMessageType};

// misc
pub use messages::DEFAULT_MAX_PAYLOAD_SIZE;

/// type aliases for the P2P Message channel
pub type P2PMessageChannelSender = Sender<P2PMessage>;
pub type P2PMessageChannelReceiver = Receiver<P2PMessage>;