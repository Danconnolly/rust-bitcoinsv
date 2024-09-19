use crate::bitcoin::BlockchainId;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use minactor::{create_actor, Actor, ActorRef, Control};
use tokio::task::JoinHandle;
use crate::BsvResult;
use crate::p2p::ACTOR_CHANNEL_SIZE;
use crate::p2p::connection::{Connection, ConnectionConfig};
use crate::p2p::envelope::{P2PMessageChannelReceiver, P2PMessageChannelSender};
use crate::p2p::manager::P2PManagerState::{Paused, Running};
use crate::p2p::manager::P2PMgrCallMessage::ReplyState;
use crate::p2p::peer::PeerAddress;
use crate::result::InternalError;

/// Configuration for the P2PManager.
#[derive(Debug, Clone)]
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
    /// Send control messages to the data channel.
    pub send_control_msgs: bool,
}

impl P2PManagerConfig {
    /// Get default [P2PManager] configuration for a particular blockchain.
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
            send_control_msgs: false,
        }
    }
}

impl Default for P2PManagerConfig {
    /// Default [P2PManager] configuration, connects to mainnet, targets 8 peers.
    fn default() -> Self {
        P2PManagerConfig::default(BlockchainId::Main)
    }
}

/// A P2PManager establishes and manages multiple P2P connections.
///
/// The P2PManager is the normal method for establishing connectivity with the Bitcoin network. When started, the
/// P2PManager will discover peers and connect to several of them. It will also create a listener to accept inbound
/// connections. Bitcoin data messages will be emitted to the data channel, where they can be acted upon
/// by other sub-systems. (todo: implement listener)
///
/// In this library we distinguish between "data" messages and "control" P2P messages. The data messages are those
/// messages which pertain to the blockchain itself, such as transaction advertisements, transactions,
/// block announcements, etc. The control messages are those messages that pertain to the establishment of the
/// connection (protoconf, setheaders, etc) and the management of the network (addr messages). The data messages
/// are sent to the data channel. By default, the control messages are not sent to this channel but this can be
/// configured. To subscribe to the data channel, use the subscribe() method. This uses the tokio::sync::broadcast
/// channel.
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
    /// The P2PManager struct is actually a handle to an actor implemented in P2PManagerActor.
    actor: ActorRef<P2PManagerActor>,
    /// The data channel
    data_channel: P2PMessageChannelSender,
}

impl P2PManager {
    /// Create a new P2PManager.
    ///
    /// This returns the P2PManager and a tokio join handle to the P2PManager actor.
    ///
    /// The join handle should be awaited at termination to ensure that the P2PManager is stopped in a normal fashion.
    pub async fn new(config: P2PManagerConfig)
                -> (P2PManager, JoinHandle<()>) {
        let (data_tx, _data_rx) = tokio::sync::broadcast::channel(ACTOR_CHANNEL_SIZE);
        let d_tx2 = data_tx.clone();
        let actor = P2PManagerActor::new(config, d_tx2);
        let (a_ref, j) = create_actor(actor).await.unwrap();
        (P2PManager {data_channel: data_tx, actor: a_ref}, j)
    }

    /// Subscribe to the data channel.
    pub fn subscribe(&self) -> P2PMessageChannelReceiver {
        self.data_channel.subscribe()
    }

    /// Stop the P2PManager, shutting down all connections and terminating all processes.
    ///
    /// The P2PManager can not be re-started after this command.
    pub async fn stop(&self) -> BsvResult<()> {
        self.actor.shutdown().await?;
        Ok(())
    }

    /// Pause the P2PManager, preventing the creation of new connections.
    ///
    /// Existing connections continue to be maintained but will not re-connect if disconnected.
    /// Incoming connections will be rejected.
    pub async fn pause(&self) -> BsvResult<()> {
        self.actor.send(P2PMgrSendMessage::Pause).await?;
        Ok(())
    }

    /// Resume the paused P2PManager.
    pub async fn resume(&self) -> BsvResult<()> {
        self.actor.send(P2PMgrSendMessage::Resume).await?;
        Ok(())
    }

    /// Get the current state of the P2PManager.
    pub async fn get_state(&self) -> BsvResult<P2PManagerState> {
        let r = self.actor.call(P2PMgrCallMessage::GetState).await?;
        if let ReplyState(s) = r? {
            Ok(s)
        } else {
            panic!("should never get here");
        }
    }
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
#[derive(Debug, Clone, PartialEq)]
enum P2PMgrSendMessage {
    /// Pause the P2PManager.
    Pause,
    /// Resume the P2PManager after it has been paused.
    Resume,
}

#[derive(Debug, Clone, PartialEq)]
enum P2PMgrCallMessage {
    /// Get the state of the P2PManager.
    GetState,
    /// Reply to GetState call.
    ReplyState(P2PManagerState),
}

/// The P2PManager initiates and manages P2P connections.
struct P2PManagerActor {
    config: P2PManagerConfig,
    state: P2PManagerState,
    data_channel: P2PMessageChannelSender,
    /// next connection id
    next_c_id: u64,
    /// configuration for connections
    connection_config: Arc<ConnectionConfig>,
    // current connections
    connections: HashMap<u64, (Connection, JoinHandle<()>)>,
    /// index of IP -> connection id
    ip_index: HashMap<IpAddr, u64>,
}

impl P2PManagerActor {
    fn new(
        config: P2PManagerConfig,
        data_channel: P2PMessageChannelSender,
    ) -> Self {
        let connection_config = Arc::new(ConnectionConfig::from(&config));
        P2PManagerActor {
            config,
            state: P2PManagerState::Starting,
            data_channel,
            next_c_id: 0,
            connection_config,
            connections: HashMap::new(),
            ip_index: HashMap::new(),
        }
    }

    /// Initiate a connection to a peer.
    async fn connect(&mut self, p: PeerAddress) {
        if let std::collections::hash_map::Entry::Vacant(e) = self.ip_index.entry(p.ip()) {
            let (c, j) = Connection::new(p.clone(), self.connection_config.clone(),
                        Some(self.data_channel.clone()));
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

/// The P2PManagerActor is an Actor from minactor.
impl Actor for P2PManagerActor {
    type SendMessage = P2PMgrSendMessage;
    type CallMessage = P2PMgrCallMessage;
    type ErrorType = InternalError;

    async fn on_initialization(&mut self) -> Control {
        // todo: if config.add_peers then start process to find dns peers
        if self.config.start_paused {
            self.state = Paused;
        } else {
            self.state = Running;
            let initial_peers = self.config.initial_peers.clone();
            for p in initial_peers {
                self.connect(p).await;
            }
        }
        Control::Ok
    }

    async fn handle_sends(&mut self, msg: Self::SendMessage) -> Control {
        match msg {
            P2PMgrSendMessage::Pause => {
                self.state = Paused;
            },
            P2PMgrSendMessage::Resume => {
                self.state = Running;
            },
        }
        Control::Ok
    }

    async fn handle_calls(&mut self, msg: Self::CallMessage) -> (Control, Result<Self::CallMessage, InternalError>) {
        match msg {
            P2PMgrCallMessage::GetState => {
                (Control::Ok, Ok(ReplyState(self.state.clone())))
            },
            _ => { panic!("should never get here"); }
        }
    }

    async fn on_shutdown(&mut self) -> Control {
        for (_, (c, j)) in self.connections.drain() {
            c.close().await;
            // todo: remove expect
            j.await.expect("Connection failed");
        }
        self.state = P2PManagerState::Stopped;
        Control::Ok
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::BlockchainId::Main;

    #[tokio::test]
    async fn start_stop_test() {
        let (h, j) = P2PManager::new(P2PManagerConfig::default(Main)).await;
        let s = h.get_state().await;
        assert!(s.is_ok());
        assert_eq!(s.unwrap(), Running);
        let _ = h.stop().await;
        j.await.expect("P2PManager failed");
    }
}
