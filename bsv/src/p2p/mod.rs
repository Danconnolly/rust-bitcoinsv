mod manager;
mod connection;
mod channel;
mod peer;
mod listener;

pub use self::manager::{P2PManagerConfig, P2PManager};

const ACTOR_CHANNEL_SIZE: usize = 100;
