mod manager;
mod connection;
mod stream;
mod peer;
mod listener;
mod messages;
mod params;


pub use self::manager::{P2PManagerConfig, P2PManager};
pub use self::peer::PeerAddress;

// size of the channel used to communicate with actors
const ACTOR_CHANNEL_SIZE: usize = 100;
