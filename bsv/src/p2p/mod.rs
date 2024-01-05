mod manager;
mod connection;
mod channel;
mod peer;
mod listener;
mod messages;

pub use self::manager::{P2PManagerConfig, P2PManager};
pub use self::peer::PeerAddress;

// size of the channel used to communicate with actors
const ACTOR_CHANNEL_SIZE: usize = 100;
