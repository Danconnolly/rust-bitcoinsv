use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};

mod node_addr;
mod version;
mod messages;
mod msg_header;
mod protoconf;
mod ping;

// the individual P2P messages
pub use node_addr::NodeAddr;
pub use ping::Ping;
pub use protoconf::Protoconf;
pub use version::Version;

// P2P message
pub use messages::{P2PMessage, P2PMessageType};

/// type aliases for the P2P Message channel
pub type P2PMessageChannelSender = Sender<Arc<P2PMessage>>;
pub type P2PMessageChannelReceiver = Receiver<Arc<P2PMessage>>;