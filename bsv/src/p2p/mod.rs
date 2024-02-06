mod manager;
mod connection;
mod stream;
mod peer;
mod listener;
mod messages;
mod params;
mod config;
mod envelope;


pub use self::manager::{P2PManager, P2PManagerConfig};
pub use self::peer::PeerAddress;

// size of the channel used to control actors
const ACTOR_CHANNEL_SIZE: usize = 100;
