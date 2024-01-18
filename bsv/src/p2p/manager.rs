use crate::bitcoin::BlockchainId;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use crate::{Error, Result};
use crate::p2p::ACTOR_CHANNEL_SIZE;
use crate::p2p::connection::{Connection, GlobalConnectionConfig};
use crate::p2p::messages::P2PMessageChannelSender;
use crate::p2p::peer::PeerAddress;


/// Configuration for the P2PManager.
pub struct P2PManagerConfig {
    /// The blockchain (mainnet, testnet, stn, regtest) to use.
    pub blockchain: BlockchainId,
    /// Whether to listen for inbound connections.
    pub listen: bool,
    /// The port to listen on, if different from the default for the blockchain.
    pub listen_port: Option<u16>,
    /// The target number of connections to maintain.
    ///
    /// The P2PManager will keep trying to add connections until this target is met.
    pub connections_target: u16,
    /// The maximum number of connections to maintain.
    ///
    /// The P2PManager will start refusing connections when this target is hit.
    pub connections_max: Option<u16>,
    /// Whether to add connections based on discovered peers.
    ///
    /// If this is false, then all peers must be manually added.
    pub add_peers: bool,
    /// Initial list of peers to which connections should be established.
    ///
    /// Note that if start_paused is true then this list is not processed.
    pub initial_peers: Vec<PeerAddress>,
    /// If true then start in the paused state.
    pub start_paused: bool,
}

impl P2PManagerConfig {
    /// Get default configuration for a particular blockchain.
    pub fn default(chain: BlockchainId) -> Self {
        P2PManagerConfig {
            blockchain: chain,
            listen: true,
            listen_port: None,
            connections_target: 8,
            connections_max: None,
            add_peers: true,
            initial_peers: Vec::new(),
            start_paused: false,
        }
    }
}

impl Default for P2PManagerConfig {
    // Default configuration, connects to mainnet, targets 8 peers.
    fn default() -> Self {
        P2PManagerConfig::default(BlockchainId::Mainnet)
    }
}

/// A P2PManager establishes and manages multiple P2P connections.
///
/// The P2PManager is the normal method for establishing connectivity with the Bitcoin network. When started, the
/// P2PManager will discover peers and connect to several of them. It will also create a listener to accept inbound
/// connections. Bitcoin data messages will be emitted to the data broadcast channel, where they can be acted upon
/// by other sub-systems. (todo: implement listener)
///
/// In this library we distinguish between "data" messages and "control" messages. The data messages are those
/// messages which pertain to the blockchain itself, such as transaction advertisements, transactions,
/// block announcements, etc. The control messages are those messages that pertain to the establishment of the
/// connection (protoconf, setheaders, etc) and the management of the network (addr messages). The data messages
/// are sent to the data broadcast channel. The control messages are only sent to the control broadcast channel if
/// this is configured. (todo: implement control channel).
///
/// A trace channel can also be configured. If configured, all sent & received P2P messages will be broadcast to
/// this channel. (todo: implement)
///
/// The status of the P2PManager can be queried at any time through the status() method and detailed status events
/// will also be sent to the status broadcast channel if it is configured.  (todo:implement)
///
/// The P2PManager can be "paused" and "resumed". In the paused state, the P2PManager will maintain existing
/// connections but it will not create new connections, re-establish broken connections, or accept new incoming
/// connections.
///
/// The P2PManager can manage many connections. In a multi-system design, we would expect one P2PManager per system,
/// with each P2PManager managing all of the connections on that system and some higher-level coordinator managing
/// the P2PManagers. This higher level coordinator has not been designed or implemented yet.
///
/// For each connection, the P2PManager creates a Connection actor which manages the connection to a single peer.
/// Each Connection actor will also create Channel actors for maintaining separate channels to the peer.
///
/// Although normally there should be only one manager per system, we allow more because its useful for testing
/// purposes.
pub struct P2PManager {
    // The P2PManager struct is actually a handle to an actor implemented in P2PManagerActor.
    sender: Sender<P2PMgrControlMessage>,
}

impl P2PManager {
    /// Create a new P2PManager.
    ///
    /// This returns the P2PManager and a tokio join handle to the P2PManager actor.
    ///
    /// The join handle should be awaited at termination to ensure that the P2PManager is stopped in a normal fashion.
    ///
    /// The P2PManager emits status messages to the status_channel and ensures that data messages are emitted to to the
    /// data_channel.
    pub fn new(config: P2PManagerConfig, status_channel: Option<P2PManagerStatusChannelSender>, msg_channel: Option<P2PMessageChannelSender>)
                -> (P2PManager, JoinHandle<()>) {
        let (tx, rx) = channel(ACTOR_CHANNEL_SIZE);
        (P2PManager { sender: tx },
         tokio::spawn(async move { P2PManagerActor::new(rx, config, status_channel, msg_channel).await }))
    }

    /// Stop the P2PManager, shutting down all connections and terminating all processes.
    ///
    /// The P2PManager can not be re-started after this command.
    pub async fn stop(&self) -> Result<()> {
        self.sender.send(P2PMgrControlMessage::Stop).await.map_err(|_| Error::Internal("Failed to send stop message".parse().unwrap()))?;
        Ok(())
    }

    /// Pause the P2PManager, preventing the creation of new connections.
    ///
    /// Existing connections continue to be maintained but will not re-connect if disconnected.
    /// Incoming connections will be rejected.
    pub async fn pause(&self) -> Result<()> {
        self.sender.send(P2PMgrControlMessage::Pause).await.map_err(|_| Error::Internal("Failed to send pause message".parse().unwrap()))?;
        Ok(())
    }

    /// Resume the paused P2PManager.
    pub async fn resume(&self) -> Result<()> {
        self.sender.send(P2PMgrControlMessage::Resume).await.map_err(|_| Error::Internal("Failed to send resume message".parse().unwrap()))?;
        Ok(())
    }

    /// Get the current state of the P2PManager.
    pub async fn get_state(&self) -> Result<P2PManagerState> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(P2PMgrControlMessage::GetState { reply: tx }).await.map_err(|_| Error::Internal("Failed to send message".parse().unwrap()))?;
        let r = rx.await.map_err(|_| Error::Internal("Failed to receive message".parse().unwrap()))?;
        Ok(r)
    }
}

/// type alias for the P2PManager status channel
pub type P2PManagerStatusChannelSender = Sender<P2PManagerStatusMessage>;

/// Status messages emitted by the P2PManager.
pub enum P2PManagerStatusMessage {
    Paused,
    Resumed,
    Stopping,
    PeerConnected,
    PeerDisconnected,
    PeerConnectionFailed,
    PeerDiscovered(PeerAddress),
}

#[derive(Debug, PartialEq, Clone)]
pub enum P2PManagerState {
    Starting,
    Running,
    Paused,
    Stopping,
    Stopped,
}

/// Internal messages that control the P2PManager.
enum P2PMgrControlMessage {
    Stop,
    Pause,
    Resume,
    GetState { reply: oneshot::Sender<P2PManagerState> },
}

/// The P2PManager initiates and manages P2P connections.
struct P2PManagerActor {
    inbox: Receiver<P2PMgrControlMessage>,
    config: P2PManagerConfig,
    state: P2PManagerState,
    status_channel: Option<P2PManagerStatusChannelSender>,
    msg_channel: Option<P2PMessageChannelSender>,
    /// next connection id
    next_c_id: u64,
    /// configuration for connections
    connection_config: Arc<GlobalConnectionConfig>,
    // current connections
    connections: HashMap<u64, (Connection, JoinHandle<()>)>,
    /// index of IP -> connection id
    ip_index: HashMap<IpAddr, u64>,
}

impl P2PManagerActor {
    async fn new(
        inbox: Receiver<P2PMgrControlMessage>,
        config: P2PManagerConfig,
        status_channel: Option<P2PManagerStatusChannelSender>,
        msg_channel: Option<P2PMessageChannelSender>,
    ) {
        let connection_config = Arc::new(GlobalConnectionConfig::default(config.blockchain));
        let mut actor = P2PManagerActor {
            inbox,
            config,
            state: P2PManagerState::Starting,
            status_channel,
            msg_channel,
            next_c_id: 0,
            connection_config,
            connections: HashMap::new(),
            ip_index: HashMap::new(),
        };
        actor.run().await
    }

    // main function
    async fn run(&mut self) {
        if self.config.start_paused {
            self.state = P2PManagerState::Paused;
        } else {
            if self.config.add_peers {
                self.start_dns_query();
            }
            self.state = P2PManagerState::Running;
            let initial_peers = self.config.initial_peers.clone();
            for p in initial_peers {
                self.connect(p).await;
            }
        }
        while self.state != P2PManagerState::Stopping {
            match self.inbox.recv().await {
                Some(msg) => match msg {
                    P2PMgrControlMessage::Pause => {
                        self.state = P2PManagerState::Paused;
                        self.send_status_msg(P2PManagerStatusMessage::Paused).await;
                    },
                    P2PMgrControlMessage::Resume => {
                        self.state = P2PManagerState::Running;
                        self.send_status_msg(P2PManagerStatusMessage::Resumed).await;
                    }
                    P2PMgrControlMessage::Stop => {
                        self.state = P2PManagerState::Stopping;
                        self.send_status_msg(P2PManagerStatusMessage::Stopping).await;
                    }
                    P2PMgrControlMessage::GetState {reply } => {
                        let _ = reply.send(self.state.clone());
                    }
                },
                None => {}
            }
        }
        // close all connections
        for (_, (c, j)) in self.connections.drain() {
            c.close().await;
            j.await.expect("Connection failed");
        }
        self.state = P2PManagerState::Stopped;
    }

    async fn send_status_msg(&self, msg: P2PManagerStatusMessage) {
        if let Some(ref tx) = self.status_channel {
            let _ = tx.send(msg).await;
        }
    }

    async fn connect(&mut self, p: PeerAddress) {
        if let std::collections::hash_map::Entry::Vacant(e) = self.ip_index.entry(p.ip()) {
            let (c, j) = Connection::new(p.clone(), self.connection_config.clone(),
                        self.msg_channel.clone());
            self.connections.insert(self.next_c_id, (c, j));
            e.insert(self.next_c_id);
            self.next_c_id += 1
        }
    }

    async fn disconnect(&mut self, p: PeerAddress) {
        if let Some(c_id) = self.ip_index.get(&p.ip()) {
            if let Some((c, j)) = self.connections.remove(c_id) {
                c.close().await;
                j.await.expect("Connection failed");
            }
        }
    }

    // start task to query the dns servers and find peers
    fn start_dns_query(&self) {} // todo
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::BlockchainId::Mainnet;

    #[tokio::test]
    async fn start_stop_test() {
        let (h, j) = P2PManager::new(P2PManagerConfig::default(Mainnet), None, None);
        let s = h.get_state().await;
        assert!(s.is_ok());
        assert_eq!(s.unwrap(), P2PManagerState::Running);
        h.stop().await;
        j.await.expect("P2PManager failed");
    }
}
