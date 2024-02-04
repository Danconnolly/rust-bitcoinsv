use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use log::trace;
use crate::bitcoin::BlockchainId;
use crate::bitcoin::BlockchainId::Mainnet;
use crate::p2p::peer::PeerAddress;
use crate::p2p::ACTOR_CHANNEL_SIZE;
use crate::p2p::stream::PeerStream;
use crate::p2p::messages::{P2PMessageChannelReceiver, P2PMessageChannelSender};
use crate::p2p::params::{DEFAULT_EXCESSIVE_BLOCK_SIZE, DEFAULT_MAX_RECV_PAYLOAD_SIZE, NetworkParams};


/// Configuration shared by all P2P Connections.
///
/// This is desired configuration, not actual configuration.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// The blockchain (mainnet, testnet, stn, regtest) to use.
    pub blockchain: BlockchainId,
    /// The number of retries to attempt when connecting to a peer, or re-connecting.
    pub retries: u8,
    /// The delay between retries, in seconds.
    pub retry_delay: u16,
    /// Should control messages be sent to the data channel?
    pub send_control_messages: bool,
    /// The maximum payload size we want to receive, using protoconf.
    pub max_recv_payload_size: u32,
    /// The excessive block size. This is the maximum size of a block that we will accept.
    pub excessive_block_size: u64,
}

impl ConnectionConfig {
    /// Get default configuration for a particular blockchain.
    pub fn default(chain: BlockchainId) -> Self {
        ConnectionConfig {
            blockchain: chain,
            retries: 5,
            retry_delay: 10,
            send_control_messages: false,
            max_recv_payload_size: DEFAULT_MAX_RECV_PAYLOAD_SIZE,
            excessive_block_size: DEFAULT_EXCESSIVE_BLOCK_SIZE,
        }
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        ConnectionConfig::default(Mainnet)
    }
}

/// A Connection represents a logical connection to a peer and it manages sending and receiving P2P messages.
///
/// The Connection can be used to establish a connectivity with a peer. Bitcoin data messages will be emitted to the
/// data channel, where they can be acted upon by other sub-systems.
///
/// In this library we distinguish between "data" messages and "control" P2P messages. The data messages are those
/// messages which pertain to the blockchain itself, such as transaction advertisements, transactions,
/// block announcements, etc. The control messages are those messages that pertain to the establishment of the
/// connection (protoconf, setheaders, etc) and the management of the network (addr messages). The data messages
/// are sent to the data channel. By default, the control messages are not sent to this channel but this can be
/// configured. To subscribe to the data channel, use the subscribe() method. This uses the tokio::sync::broadcast
/// channel. The P2P Messages are encapsulated in an Arc to avoid excessive cloning.
///
/// The Connection can be "paused" and "resumed". In the paused state, the Connection will maintain the existing
/// connection but it will not re-establish the connection if it is broken.
///
/// The P2PManager is the recommended structure for managing multiple connections.
///
/// A logical connection to a peer can consist of multiple channels which enables the separation
/// of messages based on priority and prevents the logical connection from being swamped with
/// large data messages. (todo: not implemented)
/// 
/// The Connection is actually a handle to an actor implemented in ConnectionActor.
pub struct Connection {
    // used to send connection control messages to the actor
    sender: Sender<ConnectionControlMessage>,
    /// The address to which the Connection is attempting to connect.
    pub peer: PeerAddress,
    data_channel: P2PMessageChannelSender,
}

impl Connection {
    pub fn new(peer: PeerAddress, config: Arc<ConnectionConfig>, data_channel: Option<P2PMessageChannelSender>) -> (Connection, JoinHandle<()>) {
        let (tx, rx) = channel(ACTOR_CHANNEL_SIZE);
        let p_c = peer.clone();
        let d_channel = if data_channel.is_none() {
            let (tx, _rx) = tokio::sync::broadcast::channel(ACTOR_CHANNEL_SIZE);
            tx
        } else {
            data_channel.unwrap()
        };
        let d_chan2 = d_channel.clone();
        let j = tokio::spawn(async move { ConnectionActor::new(rx, p_c, config, d_chan2).await });
        (Connection {
            sender: tx, peer, data_channel: d_channel, }, j)
    }

    /// Subscribe to the data channel.
    pub fn subscribe(&self) -> P2PMessageChannelReceiver {
        self.data_channel.subscribe()
    }

    pub async fn close(&self) {
        self.sender.send(ConnectionControlMessage::Close).await.unwrap();
    }
}

pub enum ConnectionControlMessage {
    Close,          // close the connection
    Pause,          // pause the connection, i.e. dont re-connect if it fails
}

// The actor for a connection.
//
// At the moment we only support one stream per connection, but in the future we will support multiple streams.
struct ConnectionActor {
    // the actor inbox
    inbox: Receiver<ConnectionControlMessage>,
    // the configuration for the connection, we'll need this when we support multiple channels
    config: Arc<ConnectionConfig>,
    // the channel on which to send substantive P2P messages
    data_channel: P2PMessageChannelSender,
    // number of attempts to connect
    attempts: u8,
    // the primary communication stream
    primary_stream: PeerStream,
    // the join handle for the primary channel
    primary_join: Option<JoinHandle<()>>,
    // the peer
    peer_address: PeerAddress,
    // whether the connection is paused
    paused: bool,
}

impl ConnectionActor {
    async fn new(inbox: Receiver<ConnectionControlMessage>, peer_address: PeerAddress, config: Arc<ConnectionConfig>,
                 data_channel: P2PMessageChannelSender) {
        let (stream, join_handle) = PeerStream::new(peer_address.clone(), config.clone(), data_channel.clone());
        let mut actor = ConnectionActor {
            inbox, config,
            data_channel,
            attempts: 0,
            primary_stream: stream,
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
                            self.primary_stream.close().await;
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
