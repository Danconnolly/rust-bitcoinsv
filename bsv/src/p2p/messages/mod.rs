use tokio::sync::mpsc::Sender;

mod node_addr;
mod version;
mod messages;
mod msg_header;

// the individual P2P messages
pub use node_addr::NodeAddr;
pub use version::Version;

// P2P message
pub use messages::{P2PMessage, P2PMessageType};

// misc
pub use messages::DEFAULT_MAX_PAYLOAD_SIZE;

/// type alias for the P2P Message channel
pub type P2PMessageChannelSender = Sender<P2PMessage>;
