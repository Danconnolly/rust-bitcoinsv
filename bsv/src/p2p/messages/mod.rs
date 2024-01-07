use tokio::sync::mpsc::Sender;

mod node_addr;
mod version;

pub use node_addr::NodeAddr;
pub use version::Version;


/// P2PMessages that are sent between nodes.
pub enum P2PMessage {
    Version,
    Verack,
}

/// type alias for the P2P Message channel
pub type P2PMessageChannelSender = Sender<P2PMessage>;
