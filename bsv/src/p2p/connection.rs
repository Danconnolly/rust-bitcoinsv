use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use log::trace;
use crate::bitcoin::BlockchainId;
use crate::bitcoin::BlockchainId::Mainnet;
use crate::p2p::peer::PeerAddress;
use crate::p2p::ACTOR_CHANNEL_SIZE;
use crate::p2p::channel::PeerChannel;
use crate::p2p::messages::P2PMessageChannelSender;
use crate::p2p::params::NetworkParams;


/// Configuration shared by all P2P Connections.
pub struct GlobalConnectionConfig {
    /// The blockchain (mainnet, testnet, stn, regtest) to use.
    pub blockchain: BlockchainId,
    /// The number of retries to attempt when connecting to a peer, or re-connecting.
    pub retries: u8,
    /// The delay between retries, in seconds.
    pub retry_delay: u16,
}

impl GlobalConnectionConfig {
    /// Get default configuration for a particular blockchain.
    pub fn default(chain: BlockchainId) -> Self {
        GlobalConnectionConfig {
            blockchain: chain,
            retries: 5,
            retry_delay: 10,
        }
    }
}

impl Default for GlobalConnectionConfig {
    fn default() -> Self {
        GlobalConnectionConfig::default(Mainnet)
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
    pub fn new(peer: PeerAddress, config: Arc<GlobalConnectionConfig>, data_channel: P2PMessageChannelSender) -> (Connection, JoinHandle<()>) {
        let (tx, rx) = channel(ACTOR_CHANNEL_SIZE);
        let p_c = peer.clone();
        let j = tokio::spawn(async move { ConnectionActor::new(rx, p_c, config, data_channel).await });
        (Connection { sender: tx, peer }, j)
    }

    pub async fn close(&self) {
        self.sender.send(ConnectionControlMessage::Close).await.unwrap();
    }
}

pub enum ConnectionControlMessage {
    Close,          // close the connection
    Pause,          // pause the connection, i.e. dont re-connect if it fails
}

/// The actor for a connection.
///
/// At the moment we only support one channel per connection, but in the future we will support multiple channels.
struct ConnectionActor {
    // the actor inbox
    inbox: Receiver<ConnectionControlMessage>,
    // the configuration for the connection, we'll need this when we support multiple channels
    config: Arc<GlobalConnectionConfig>,
    // the network parameters
    network_params: NetworkParams,
    // the channel on which to send substantive P2P messages
    data_channel: P2PMessageChannelSender,
    // number of attempts to connect
    attempts: u8,
    // the primary communication channel
    primary_channel: PeerChannel,
    // the join handle for the primary channel
    primary_join: Option<JoinHandle<()>>,
    // the peer
    peer_address: PeerAddress,
    // whether the connection is paused
    paused: bool,
}

impl ConnectionActor {
    async fn new(inbox: Receiver<ConnectionControlMessage>, peer_address: PeerAddress, config: Arc<GlobalConnectionConfig>, data_channel: P2PMessageChannelSender) {
        let network_params =  NetworkParams::from(config.blockchain);
        let (channel, join_handle) = PeerChannel::new(peer_address.clone(), config.clone(), network_params.clone(), data_channel.clone());
        let mut actor = ConnectionActor {
            inbox, config,
            network_params,
            data_channel: data_channel,
            attempts: 0,
            primary_channel: channel,
            primary_join: Some(join_handle),
            peer_address,
            paused: false,
        };
        actor.run().await;
    }
    
    async fn run(&mut self) {
        trace!("ConnectionActor started.");
        loop {
            tokio::select! {
                Some(msg) = self.inbox.recv() => {
                    match msg {
                        ConnectionControlMessage::Close => {
                            self.primary_channel.close().await;
                            let h = self.primary_join.take().unwrap();
                            h.await.unwrap();
                            break;
                        },
                        ConnectionControlMessage::Pause => {
                            self.paused = true;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    
    

    // todo: add some tests

    // #[tokio::test]
    // async fn start_stop_test() {
    //     let address = PeerAddress::new("127.0.0.1:8321".parse().unwrap());
    //     let (h, j) = Connection::new(address, Arc::new(GlobalConnectionConfig::default(Mainnet)), None);
    //     h.close().await;
    //     j.await.expect("Connection failed");
    // }
}
