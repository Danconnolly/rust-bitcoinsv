use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use crate::bitcoin::BlockchainId;
use crate::p2p::peer::PeerAddress;
use crate::p2p::ACTOR_CHANNEL_SIZE;
use crate::p2p::messages::P2PMessageChannelSender;


/// Configuration for a P2P Connection.
pub struct ConnectionConfig {
    /// The blockchain (mainnet, testnet, stn, regtest) to use.
    pub blockchain: BlockchainId,
}

impl ConnectionConfig {
    /// Get default configuration for a particular blockchain.
    pub fn default(chain: BlockchainId) -> Self {
        ConnectionConfig {
            blockchain: chain,
        }
    }
}

/// A Connection represents a logical connection to a peer and it manages sending and receiving P2P messages.
///
/// A logical connection to a peer can consist of multiple channels which enables the separation
/// of messages based on priority and prevents the logical connection from being swamped with
/// large data messages. 
/// 
/// The Connection is actually a handle to an actor implemented in ConnectionActor.
pub struct Connection {
    sender: Sender<ConnectionControlMessage>,
    peer: PeerAddress,
}

impl Connection {
    pub fn new(peer: PeerAddress, config: Arc<ConnectionConfig>, msg_channel: Option<P2PMessageChannelSender>) -> (Connection, JoinHandle<()>) {
        let (tx, rx) = channel(ACTOR_CHANNEL_SIZE);
        let j = tokio::spawn(async move { ConnectionActor::new(rx, config, msg_channel).await });
        (Connection { sender: tx, peer }, j)
    }

    pub async fn close(&self) {
        self.sender.send(ConnectionControlMessage::Close).await.unwrap();
    }
}

pub enum ConnectionControlMessage {
    Close,
}

struct ConnectionActor {
    inbox: Receiver<ConnectionControlMessage>,
    config: Arc<ConnectionConfig>,
    msg_channel: Option<P2PMessageChannelSender>,
}

impl ConnectionActor {
    async fn new(inbox: Receiver<ConnectionControlMessage>, config: Arc<ConnectionConfig>, msg_channel: Option<P2PMessageChannelSender>) {
        let mut actor = ConnectionActor { inbox, config, msg_channel };
        actor.run().await;
    }
    
    async fn run(&mut self) {
        loop {
            tokio::select! {
                Some(msg) = self.inbox.recv() => {
                    match msg {
                        ConnectionControlMessage::Close => {
                            break;
                        }
                    }
                }
            }
        }
    }
}
