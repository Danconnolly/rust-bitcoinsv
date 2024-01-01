use crate::base::Blockchain;
use crate::util::ACTOR_CHANNEL_SIZE;
use std::collections::HashMap;
use std::net::IpAddr;
use tokio;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use crate::p2p::peer::Peer;

/// Configuration for the P2PManager.
pub struct P2PManagerConfig {
    /// The blockchain (mainnet, testnet, stn, regtest) to use.
    pub blockchain: Blockchain,
    /// Whether the P2PManager should start in the paused state.
    pub paused: bool,
    /// The target number of connections to maintain.
    pub connections_target: u16,
    /// The maximum number of connections to maintain.
    pub connections_max: u16,
    /// Whether to query the DNS Seeds.
    pub query_dns: bool,
    /// Whether to add connections based on advertisements.
    pub add_addr_peers: bool,
    /// Peers to which connections should be established.
    pub known_peers: Vec<Peer>,
}

impl P2PManagerConfig {
    pub fn default(chain: Blockchain) -> Self {
        P2PManagerConfig {
            blockchain: chain,
            paused: false,
            connections_target: 10, // default small, designed for small client applications
            connections_max: 20,    // default small, designed for small client applications
            query_dns: true,
            add_addr_peers: true,
            known_peers: Vec::new(),
        }
    }
}

impl Default for P2PManagerConfig {
    // Default connects to mainnet
    fn default() -> Self {
        P2PManagerConfig::default(Blockchain::Mainnet)
    }
}

pub struct P2PManager {
    /// A P2PManager manages P2P connections.
    ///
    /// The P2PManager struct is actually a handle to an actor implemented in P2PManagerActor.
    sender: Sender<P2PMgrMessage>,
}

impl P2PManager {
    /// Create a new P2PManager. There should be only one manager per system but we allow more
    /// because its useful for testing purposes.
    pub fn new(config: P2PManagerConfig) -> (P2PManager, JoinHandle<()>) {
        let (tx, rx) = channel(ACTOR_CHANNEL_SIZE);
        let t2 = tx.clone();
        let j = tokio::spawn(async move { P2PManagerActor::new(rx, t2, config).await });
        (P2PManager { sender: tx }, j)
    }

    /// Stop the P2PManager, shutting down all connections.
    pub async fn stop(&self) {
        self.sender.send(P2PMgrMessage::Stop).await;
    }

    /// Pause the P2PManager, preventing the creation of new connections.
    /// Existing connections continue to be maintained but will not re-connect if disconnected.
    pub async fn pause(&self) {
        self.sender.send(P2PMgrMessage::Pause).await;
    }

    /// Resume the paused P2PManager.
    pub async fn resume(&self) {
        self.sender.send(P2PMgrMessage::Resume).await;
    }

    /// Get the current state of the P2PManager.
    pub async fn state(&self) -> P2PManagerState {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(P2PMgrMessage::GetState { reply: tx })
            .await;
        rx.await.unwrap()
    }
}

pub enum P2PMgrMessage {
    Stop,
    Pause,
    Resume,
    GetState { reply: oneshot::Sender<P2PManagerState> },
}

#[derive(Debug, PartialEq, Clone)]
pub enum P2PManagerState {
    Starting,
    Running,
    Paused,
    Stopping,
    Stopped,
}

/// The P2PManager initiates and manages P2P connections.
struct P2PManagerActor {
    inbox: Receiver<P2PMgrMessage>,
    outbox: Sender<P2PMgrMessage>,
    config: P2PManagerConfig,
    state: P2PManagerState,
    /// next connection id
    next_c_id: u64,
    // connections
    // conns: HashMap<u64, (ConnectionHandle, JoinHandle<()>)>,
    /// index of IP -> connection id
    ip_index: HashMap<IpAddr, u64>,
}

impl P2PManagerActor {
    async fn new(
        inbox: Receiver<P2PMgrMessage>,
        outbox: Sender<P2PMgrMessage>,
        config: P2PManagerConfig,
    ) {
        let mut actor = P2PManagerActor {
            inbox,
            outbox,
            config,
            state: P2PManagerState::Starting,
            next_c_id: 0,
            ip_index: HashMap::new(),
        };
        actor.run().await
    }

    // main function
    pub async fn run(&mut self) {
        if self.config.paused {
            self.state = P2PManagerState::Paused;
        } else {
            if self.config.query_dns {
                self.start_dns_query();
            }
            self.state = P2PManagerState::Running;
        }
        // open known connections
        // for p in self.config.known_peers {
        //     self.connect(&p).await;
        // }
        while self.state != P2PManagerState::Stopping {
            match self.inbox.recv().await {
                Some(msg) => match msg {
                    P2PMgrMessage::Pause => {
                        self.state = P2PManagerState::Paused;
                    },
                    P2PMgrMessage::Resume => {
                        self.state = P2PManagerState::Running;
                    }
                    P2PMgrMessage::Stop => {
                        self.state = P2PManagerState::Stopping;
                    }
                    P2PMgrMessage::GetState {reply } => {
                        let _ = reply.send(self.state.clone());
                    }
                },
                None => {}
            }
        }
        // todo: close all connections
        self.state = P2PManagerState::Stopped;
    }

    // start task to query the dns servers and find peers
    fn start_dns_query(&self) {} // todo

    // async fn connect(&mut self, p: &SocketAddr) {
    //     if ! self.ip_index.contains_key(&p.ip()) {
    //         let (c, j) = ConnectionHandle::new(self.next_c_id, self.config.blockchain, p.clone(), self.conn_config.clone()).await;
    //         self.conns.insert(self.next_c_id, (c, j));
    //         self.ip_index.insert(p.ip(), self.next_c_id);
    //         self.next_c_id += 1
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::Blockchain::Mainnet;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn simple_tests() {
        let (h, j) = P2PManager::new(P2PManagerConfig::default(Mainnet));
        let s = h.state().await;
        assert_eq!(s, P2PManagerState::Running);
        h.stop().await;
        j.await;
    }
}
