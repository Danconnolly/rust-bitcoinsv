use crate::bitcoin::BlockchainId;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use crate::p2p::ACTOR_CHANNEL_SIZE;
use crate::p2p::connection::{Connection, GlobalConnectionConfig};
use crate::p2p::messages::P2PMessageChannelSender;
use crate::p2p::peer::PeerAddress;


/// Configuration for the P2PManager.
pub struct P2PManagerConfig {
    /// The blockchain (mainnet, testnet, stn, regtest) to use.
    pub blockchain: BlockchainId,
    /// Whether the P2PManager should start in the paused state.
    pub paused: bool,
    /// The target number of connections to maintain.
    pub connections_target: u16,
    /// The maximum number of connections to maintain.
    pub connections_max: Option<u16>,
    /// Whether to add connections based on discovered peers.
    pub add_peers: bool,
    /// Initial list of peers to which connections should be established.
    ///
    /// Note that if paused is true then this list is not processed.
    pub initial_peers: Vec<PeerAddress>,
}

impl P2PManagerConfig {
    /// Get default configuration for a particular blockchain.
    pub fn default(chain: BlockchainId) -> Self {
        P2PManagerConfig {
            blockchain: chain,
            paused: false,
            connections_target: 8,
            connections_max: None,
            add_peers: true,
            initial_peers: Vec::new(),
        }
    }
}

impl Default for P2PManagerConfig {
    // default configuration, connects to mainnet.
    fn default() -> Self {
        P2PManagerConfig::default(BlockchainId::Mainnet)
    }
}

/// A P2PManager manages P2P connections.
///
/// For each connection, the P2PManager creates a Connection actor which manages the connection.
///
/// The P2PManager emits status messages to a status channel and the Connection actors emit P2P messages to a
/// message channel.
///
/// The P2PManager can manage many connections. In a multi-system design, we would expect one P2PManager per system,
/// with each P2PManager managing all of the connections on that system and some higher-level coordinator managing
/// the P2PManagers.
///
/// The P2PManager does not manage the list of peers, this is expected to be managed elsewhere.
///
/// Although normally there should be only one manager per system but we allow more because its useful for testing
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
    /// The join handle should be awaited at termination to ensure that the P2PManager is stopped.
    ///
    /// The P2PManager emits status messages to the status_channel and ensures that P2P messages are emitted to to the
    /// msg_channel.
    pub fn new(config: P2PManagerConfig, status_channel: Option<P2PManagerStatusChannelSender>, msg_channel: Option<P2PMessageChannelSender>)
                -> (P2PManager, JoinHandle<()>) {
        let (tx, rx) = channel(ACTOR_CHANNEL_SIZE);
        (P2PManager { sender: tx },
         tokio::spawn(async move { P2PManagerActor::new(rx, config, status_channel, msg_channel).await }))
    }

    /// Stop the P2PManager, shutting down all connections.
    pub async fn stop(&self) {
        self.sender.send(P2PMgrControlMessage::Stop).await.expect("P2PManager::stop failed");
    }

    /// Pause the P2PManager, preventing the creation of new connections.
    /// Existing connections continue to be maintained but will not re-connect if disconnected.
    pub async fn pause(&self) {
        self.sender.send(P2PMgrControlMessage::Pause).await.expect("P2PManager::pause failed");
    }

    /// Resume the paused P2PManager.
    pub async fn resume(&self) {
        self.sender.send(P2PMgrControlMessage::Resume).await.expect("P2PManager::resume failed");
    }

    /// Get the current state of the P2PManager.
    pub async fn get_state(&self) -> P2PManagerState {
        let (tx, rx) = oneshot::channel();
        self.sender.send(P2PMgrControlMessage::GetState { reply: tx }).await.expect("P2PManager::get_state failed");
        rx.await.unwrap()
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
        if self.config.paused {
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
        if ! self.ip_index.contains_key(&p.ip()) {
            let (c, j) = Connection::new(p.clone(), self.connection_config.clone(),
                        self.msg_channel.clone());
            self.connections.insert(self.next_c_id, (c, j));
            self.ip_index.insert(p.ip(), self.next_c_id);
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
        assert_eq!(s, P2PManagerState::Running);
        h.stop().await;
        j.await.expect("P2PManager failed");
    }
}
