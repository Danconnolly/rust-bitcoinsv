use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use log::trace;
use tokio::sync::RwLock;
use uuid::Uuid;
use crate::bitcoin::BlockchainId;
use crate::bitcoin::BlockchainId::Main;
use crate::p2p::peer::PeerAddress;
use crate::p2p::{P2PManagerConfig, ACTOR_CHANNEL_SIZE};
use crate::p2p::envelope::{P2PMessageChannelReceiver, P2PMessageChannelSender};
use crate::p2p::stream::{PeerStream, StreamConfig};
use crate::p2p::params::{DEFAULT_EXCESSIVE_BLOCK_SIZE, DEFAULT_MAX_RECV_PAYLOAD_SIZE};


/// Configuration shared by all P2P Connections.
///
/// This is the desired configuration.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// The blockchain (mainnet, testnet, stn, regtest) to use.
    pub blockchain: BlockchainId,
    /// The number of retries to attempt when connecting to a peer, or re-connecting. Default is 5.
    pub retries: u8,
    /// The delay between retries, in seconds. Default is 10 seconds.
    pub retry_delay: u16,
    /// Should control messages be sent to the data channel?
    pub send_control_messages: bool,
    /// The maximum payload size we want to receive, using protoconf.
    /// The default for this is DEFAULT_MAX_RECV_PAYLOAD_SIZE (200MB).
    // Note that although we have this as u64, the maximum is really u32.
    pub max_recv_payload_size: u64,
    /// The excessive block size. This is the maximum size of a block that we will accept.
    /// The default for this is DEFAULT_EXCESSIVE_BLOCK_SIZE (10GB).
    pub excessive_block_size: u64,
}

impl ConnectionConfig {
    /// Get default configuration for a particular blockchain.
    pub fn default_for(chain: BlockchainId) -> Self {
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
    /// Defaults for the ConnectionConfig.
    fn default() -> Self {
        ConnectionConfig::default_for(Main)
    }
}

impl From<&P2PManagerConfig> for ConnectionConfig {
    /// Enable the ConnectionConfig to be derived from a [P2PManagerConfig].
    fn from(value: &P2PManagerConfig) -> Self {
        ConnectionConfig {
            blockchain: value.blockchain,
            send_control_messages: value.send_control_msgs,
            ..Default::default()
        }
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
/// configured (using send_control_messages in ConnectionConfig). To subscribe to the data channel,
/// use the subscribe() method. This uses the tokio::sync::broadcast
/// channel. The P2P Messages are encapsulated in an Arc to avoid duplicating data.
///
/// Each connection is assigned a unique connection id. This is a random UUID. The connection id is assigned when
/// the Connection struct is created and will remain the same for multiple attempts to connect.
///
/// The Connection can be "paused" and "resumed". In the paused state, the Connection will maintain the existing
/// connection but it will not re-establish the connection if it is broken.
///
/// The P2PManager is the recommended structure for managing multiple connections.
///
/// A logical connection to a peer can consist of multiple streams which enables the separation
/// of messages based on priority and prevents the logical connection from being swamped with
/// large data messages. (todo: not implemented)
/// 
/// The Connection is actually a handle to an actor implemented in ConnectionActor.
pub struct Connection {
    /// The address to which the Connection is attempting to connect.
    pub peer: PeerAddress,
    /// The connection id. A random id is assigned for every connection, which can consist of multiple streams.
    pub connection_id: Uuid,
    // The broadcast channel on which substantive P2P messages are sent.
    data_channel: P2PMessageChannelSender,
    // used to send connection control messages to the actor
    sender: Sender<ConnectionControlMessage>,
}

impl Connection {
    pub fn new(peer: PeerAddress, config: Arc<ConnectionConfig>, data_channel: Option<P2PMessageChannelSender>) -> (Connection, JoinHandle<()>) {
        // actor channel
        let (tx, rx) = channel(ACTOR_CHANNEL_SIZE);
        // data channel
        let d_channel = if data_channel.is_none() {
            let (tx, _rx) = tokio::sync::broadcast::channel(ACTOR_CHANNEL_SIZE);
            tx
        } else {
            data_channel.unwrap()
        };
        let d_chan2 = d_channel.clone();
        let p_c = peer.clone();
        let connection_id = Uuid::new_v4();
        let c_id2 = connection_id.clone();
        let j = tokio::spawn(async move { ConnectionActor::new(rx, p_c, c_id2, config, d_chan2).await });
        (Connection { peer, connection_id, data_channel: d_channel, sender: tx, }, j)
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
    // unique id for the connection
    connection_id: Uuid,
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
    // the configuration of the primary stream
    primary_config: Arc<RwLock<StreamConfig>>,
    // the peer
    peer_address: PeerAddress,
    // whether the connection is paused, default false
    paused: bool,
}

impl ConnectionActor {
    async fn new(inbox: Receiver<ConnectionControlMessage>, peer_address: PeerAddress, connection_id: Uuid,
                 config: Arc<ConnectionConfig>, data_channel: P2PMessageChannelSender) {
        // make the first stream
        let stream_config = Arc::new(RwLock::new(StreamConfig::new(&config, &peer_address.peer_id, &connection_id)));
        let (stream, join_handle) = PeerStream::new(peer_address.clone(), stream_config.clone(), data_channel.clone()).await.unwrap();  // todo: remove unwrap
        // make the actor
        let mut actor = ConnectionActor {
            inbox, connection_id, config, data_channel, attempts: 0, primary_stream: stream, primary_join: Some(join_handle),
            primary_config: stream_config, peer_address, paused: false,
        };
        // run the actor
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
