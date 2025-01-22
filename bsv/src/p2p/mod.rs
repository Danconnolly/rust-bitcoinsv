//! The p2p module provides capabilities for using the Bitcoin P2P Network.
//!
//! Although this network is going to be superseded by the Mandala Upgrade, it will continue to play
//! an important role until all users have upgraded.
mod channel;
mod connection;
mod envelope;
mod listener;
mod manager;
mod messages;
mod params;
mod peer;

pub use self::connection::{Connection, ConnectionConfig, ConnectionControlMessage};
pub use self::manager::{P2PManager, P2PManagerConfig};
pub use self::peer::PeerAddress;

// size of the channel used to control actors
// todo: to be removed
const ACTOR_CHANNEL_SIZE: usize = 100;
