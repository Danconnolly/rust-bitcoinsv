use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use crate::base::Blockchain;
use crate::p2p::peer::Peer;
use crate::util::ACTOR_CHANNEL_SIZE;


/// Configuration for a P2P Connection.
pub struct ConnectionConfig {
    /// The blockchain (mainnet, testnet, stn, regtest) to use.
    pub blockchain: Blockchain,
}

/// A Connection represents a logical connection to a peer and it manages sending and receiving P2P messages.
///
/// A logical connection to a peer can consist of multiple channels which enables the separation
/// of messages based on priority and prevents the logical connection from being swamped with
/// large data messages. 
/// 
/// The Connection is actually a handle to an actor implemented in ConnectionActor.
pub struct Connection {
    sender: Sender<ConnectionMessage>,
    peer: Peer,
}

impl Connection {
    pub fn new(peer: Peer, config: ConnectionConfig) -> (Connection, JoinHandle<()>) {
        let (tx, rx) = channel(ACTOR_CHANNEL_SIZE);
        let j = tokio::spawn(async move { ConnectionActor::new(rx, config).await });
        (Connection { sender: tx, peer }, j)
    }
}

pub enum ConnectionMessage {

}

struct ConnectionActor {
    inbox: Receiver<ConnectionMessage>,
    config: ConnectionConfig,
}

impl ConnectionActor {
    async fn new(inbox: Receiver<ConnectionMessage>, config: ConnectionConfig) {
        let mut actor = ConnectionActor { inbox, config };
        actor.run().await;
    }
    
    async fn run(&mut self) {}
}
