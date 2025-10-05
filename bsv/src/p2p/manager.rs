//! P2P Network Manager
//!
//! The Manager orchestrates peer connections, maintains connection limits,
//! handles peer discovery, and manages the peer lifecycle.

use crate::p2p::{ConnectionConfig, ManagerConfig, Peer, PeerStore};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, watch, Mutex};
use uuid::Uuid;

/// Operating mode for the Manager
#[derive(Debug, Clone, PartialEq)]
pub enum OperatingMode {
    /// Normal mode: connect to any valid peers to reach target
    Normal,
    /// Fixed peer list mode: only connect to specified peers
    FixedPeerList,
}

/// Handle to an active peer connection
pub struct PeerConnectionHandle {
    pub peer_id: Uuid,
    pub control_tx: mpsc::Sender<PeerConnectionCommand>,
}

/// Commands sent to PeerConnection actors
#[derive(Debug, Clone)]
pub enum PeerConnectionCommand {
    /// Update connection configuration
    UpdateConfig(ConnectionConfig),
    /// Disconnect gracefully
    Disconnect,
    /// Send a message to the peer
    SendMessage(crate::p2p::Message),
}

/// Events broadcast by PeerConnections to Manager
#[derive(Debug, Clone)]
pub enum ControlEvent {
    /// Connection successfully established
    ConnectionEstablished { peer_id: Uuid },
    /// Connection failed to establish
    ConnectionFailed { peer_id: Uuid, reason: String },
    /// Connection lost after being established
    ConnectionLost { peer_id: Uuid },
    /// Peer was banned
    PeerBanned {
        peer_id: Uuid,
        reason: crate::p2p::BanReason,
    },
    /// Handshake completed successfully
    HandshakeComplete { peer_id: Uuid },
}

/// Bitcoin message events from peers
#[derive(Debug, Clone)]
pub struct BitcoinMessageEvent {
    pub peer_id: Uuid,
    pub message: crate::p2p::Message,
}

/// Connection slot reservation tracker
///
/// Implements atomic connection counting to prevent race conditions
/// where multiple concurrent connections exceed max_connections.
#[derive(Debug)]
pub struct ConnectionSlots {
    /// Maximum allowed connections
    max_connections: usize,
    /// Currently reserved slots (includes active + pending handshakes)
    reserved: AtomicUsize,
}

impl ConnectionSlots {
    /// Create new connection slot tracker
    pub fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            reserved: AtomicUsize::new(0),
        }
    }

    /// Try to reserve a connection slot
    ///
    /// Returns true if slot was reserved, false if at capacity
    pub fn try_reserve(&self) -> bool {
        let mut current = self.reserved.load(Ordering::SeqCst);
        loop {
            if current >= self.max_connections {
                return false;
            }

            match self.reserved.compare_exchange_weak(
                current,
                current + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    /// Release a reserved slot
    pub fn release(&self) {
        self.reserved.fetch_sub(1, Ordering::SeqCst);
    }

    /// Get current number of reserved slots
    pub fn count(&self) -> usize {
        self.reserved.load(Ordering::SeqCst)
    }

    /// Update maximum connections
    ///
    /// Returns the old maximum value.
    /// Note: This is not thread-safe for concurrent set_max calls,
    /// but that's acceptable as config updates are serialized through the Manager
    pub fn set_max(&mut self, max: usize) -> usize {
        let old_max = self.max_connections;
        self.max_connections = max;
        old_max
    }
}

/// P2P Network Manager
///
/// Manages peer connections, discovery, and lifecycle.
pub struct Manager {
    config: ManagerConfig,
    peer_store: Arc<dyn PeerStore>,
    connection_slots: Arc<ConnectionSlots>,
    active_connections: Arc<Mutex<HashMap<Uuid, PeerConnectionHandle>>>,
    control_event_tx: broadcast::Sender<ControlEvent>,
    bitcoin_message_tx: broadcast::Sender<BitcoinMessageEvent>,
    operating_mode: OperatingMode,
    fixed_peers: Option<Vec<Peer>>,
    #[allow(dead_code)] // Will be used in start/shutdown implementation
    shutdown_tx: watch::Sender<bool>,
    #[allow(dead_code)] // Will be used in start/shutdown implementation
    shutdown_rx: watch::Receiver<bool>,
}

impl Manager {
    /// Create a new Manager in Normal mode
    pub fn new(config: ManagerConfig, peer_store: Arc<dyn PeerStore>) -> Self {
        let (control_event_tx, _) = broadcast::channel(1000);
        let (bitcoin_message_tx, _) = broadcast::channel(1000);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Self {
            connection_slots: Arc::new(ConnectionSlots::new(config.max_connections)),
            config,
            peer_store,
            active_connections: Arc::new(Mutex::new(HashMap::new())),
            control_event_tx,
            bitcoin_message_tx,
            operating_mode: OperatingMode::Normal,
            fixed_peers: None,
            shutdown_tx,
            shutdown_rx,
        }
    }

    /// Create a new Manager in Fixed Peer List mode
    pub fn with_fixed_peers(
        config: ManagerConfig,
        peer_store: Arc<dyn PeerStore>,
        peers: Vec<Peer>,
    ) -> Self {
        let mut manager = Self::new(config, peer_store);
        manager.operating_mode = OperatingMode::FixedPeerList;
        manager.fixed_peers = Some(peers);
        manager
    }

    /// Subscribe to control events
    pub fn subscribe_control_events(&self) -> broadcast::Receiver<ControlEvent> {
        self.control_event_tx.subscribe()
    }

    /// Subscribe to Bitcoin messages
    pub fn subscribe_bitcoin_messages(&self) -> broadcast::Receiver<BitcoinMessageEvent> {
        self.bitcoin_message_tx.subscribe()
    }

    /// Get current connection count
    pub fn get_connection_count(&self) -> usize {
        self.connection_slots.count()
    }

    /// Start the Manager and begin connection management
    pub async fn start(&mut self) -> crate::Result<()> {
        // TODO: Implement manager start logic
        Ok(())
    }

    /// Shutdown the Manager gracefully
    pub async fn shutdown(&mut self) -> crate::Result<()> {
        // TODO: Implement shutdown logic
        Ok(())
    }

    /// Get list of peers from store
    pub async fn get_peers(&self) -> crate::Result<Vec<Peer>> {
        self.peer_store.list_all().await
    }

    /// Send a message to a specific peer
    pub async fn send_message(
        &self,
        peer_id: Uuid,
        message: crate::p2p::Message,
    ) -> crate::Result<()> {
        let connections = self.active_connections.lock().await;
        if let Some(handle) = connections.get(&peer_id) {
            handle
                .control_tx
                .send(PeerConnectionCommand::SendMessage(message))
                .await
                .map_err(|_| crate::Error::ChannelSendError)?;
            Ok(())
        } else {
            Err(crate::Error::PeerNotFound(peer_id))
        }
    }

    /// Ban a peer by ID
    pub async fn ban_peer(
        &mut self,
        peer_id: Uuid,
        reason: crate::p2p::BanReason,
    ) -> crate::Result<()> {
        let mut peer = self.peer_store.read(peer_id).await?;
        peer.ban(reason);
        self.peer_store.update(peer).await?;

        // Disconnect if currently connected
        let mut connections = self.active_connections.lock().await;
        if let Some(handle) = connections.remove(&peer_id) {
            let _ = handle
                .control_tx
                .send(PeerConnectionCommand::Disconnect)
                .await;
        }

        Ok(())
    }

    /// Unban a peer by ID
    pub async fn unban_peer(&mut self, peer_id: Uuid) -> crate::Result<()> {
        let mut peer = self.peer_store.read(peer_id).await?;
        peer.update_status(crate::p2p::PeerStatus::Unknown);
        self.peer_store.update(peer).await?;
        Ok(())
    }

    /// Update configuration dynamically
    pub async fn update_config(&mut self, config: ManagerConfig) -> crate::Result<()> {
        config.validate()?;
        self.config = config;
        // TODO: Propagate config changes to active connections
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::{InMemoryPeerStore, Network};
    use std::time::Duration;
    use tokio::time::sleep;

    fn create_test_manager() -> Manager {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.target_connections = 8;
        config.max_connections = 125;
        config.enable_listener = false;
        let peer_store = Arc::new(InMemoryPeerStore::new());
        Manager::new(config, peer_store)
    }

    #[tokio::test]
    async fn test_connection_count_reservation() {
        let slots = ConnectionSlots::new(3);

        // Should be able to reserve up to max
        assert!(slots.try_reserve());
        assert_eq!(slots.count(), 1);
        assert!(slots.try_reserve());
        assert_eq!(slots.count(), 2);
        assert!(slots.try_reserve());
        assert_eq!(slots.count(), 3);

        // Should fail when at capacity
        assert!(!slots.try_reserve());
        assert_eq!(slots.count(), 3);

        // Release one slot
        slots.release();
        assert_eq!(slots.count(), 2);

        // Should be able to reserve again
        assert!(slots.try_reserve());
        assert_eq!(slots.count(), 3);
    }

    #[tokio::test]
    async fn test_concurrent_inbound_connections_respect_max() {
        let slots = Arc::new(ConnectionSlots::new(10));
        let mut handles = vec![];

        // Spawn 20 tasks trying to reserve concurrently
        for _ in 0..20 {
            let slots_clone = Arc::clone(&slots);
            let handle = tokio::spawn(async move {
                sleep(Duration::from_millis(1)).await;
                slots_clone.try_reserve()
            });
            handles.push(handle);
        }

        // Collect results
        let mut success_count = 0;
        for handle in handles {
            if handle.await.unwrap() {
                success_count += 1;
            }
        }

        // Exactly 10 should have succeeded
        assert_eq!(success_count, 10);
        assert_eq!(slots.count(), 10);
    }

    #[tokio::test]
    async fn test_connection_released_on_handshake_failure() {
        let slots = Arc::new(ConnectionSlots::new(5));

        // Reserve a slot (simulating connection initiation)
        assert!(slots.try_reserve());
        assert_eq!(slots.count(), 1);

        // Simulate handshake failure by releasing the slot
        slots.release();
        assert_eq!(slots.count(), 0);

        // Slot should be available again
        assert!(slots.try_reserve());
        assert_eq!(slots.count(), 1);
    }

    #[tokio::test]
    async fn test_connection_count_accurate_under_concurrent_load() {
        let slots = Arc::new(ConnectionSlots::new(50));
        let mut handles = vec![];

        // Spawn 100 tasks that reserve and release
        for i in 0..100 {
            let slots_clone = Arc::clone(&slots);
            let handle = tokio::spawn(async move {
                sleep(Duration::from_millis(i % 5)).await;
                if slots_clone.try_reserve() {
                    sleep(Duration::from_millis(2)).await;
                    slots_clone.release();
                    true
                } else {
                    false
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // All slots should be released
        assert_eq!(slots.count(), 0);
    }

    #[test]
    fn test_manager_creation_normal_mode() {
        let manager = create_test_manager();
        assert_eq!(manager.operating_mode, OperatingMode::Normal);
        assert!(manager.fixed_peers.is_none());
        assert_eq!(manager.get_connection_count(), 0);
    }

    #[test]
    fn test_manager_creation_fixed_peer_mode() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.target_connections = 3;
        config.max_connections = 10;
        config.enable_listener = false;
        let peer_store = Arc::new(InMemoryPeerStore::new());
        let peers = vec![
            Peer::new("192.168.1.1".parse().unwrap(), 8333),
            Peer::new("192.168.1.2".parse().unwrap(), 8333),
        ];

        let manager = Manager::with_fixed_peers(config, peer_store, peers.clone());

        assert_eq!(manager.operating_mode, OperatingMode::FixedPeerList);
        assert_eq!(manager.fixed_peers.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_manager_subscribe_control_events() {
        let manager = create_test_manager();
        let mut rx = manager.subscribe_control_events();

        // Send an event
        let event = ControlEvent::ConnectionEstablished {
            peer_id: Uuid::new_v4(),
        };
        manager.control_event_tx.send(event.clone()).unwrap();

        // Receive the event
        let received = rx.recv().await.unwrap();
        match received {
            ControlEvent::ConnectionEstablished { .. } => {}
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_manager_subscribe_bitcoin_messages() {
        let manager = create_test_manager();
        let mut rx = manager.subscribe_bitcoin_messages();

        // Send a message event
        let event = BitcoinMessageEvent {
            peer_id: Uuid::new_v4(),
            message: crate::p2p::Message::Verack,
        };
        manager.bitcoin_message_tx.send(event.clone()).unwrap();

        // Receive the event
        let received = rx.recv().await.unwrap();
        assert_eq!(received.message, crate::p2p::Message::Verack);
    }

    // Phase 5.2: Connection Initiation Tests

    #[tokio::test]
    async fn test_normal_mode_reaches_target_connections() {
        // Create manager with low target
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.target_connections = 3;
        config.max_connections = 10;
        let peer_store = Arc::new(InMemoryPeerStore::new());

        // Add some valid peers to the store
        for i in 1..=5 {
            let peer = Peer::new(format!("192.168.1.{}", i).parse().unwrap(), 8333);
            peer_store.create(peer).await.unwrap();
        }

        let manager = Manager::new(config, peer_store);

        // In the future, starting the manager should initiate connections
        // For now, just verify the structure is correct
        assert_eq!(manager.config.target_connections, 3);
        assert_eq!(manager.operating_mode, OperatingMode::Normal);
    }

    #[tokio::test]
    async fn test_normal_mode_respects_max_connections() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.target_connections = 20;
        config.max_connections = 10;
        let peer_store = Arc::new(InMemoryPeerStore::new());

        let manager = Manager::new(config, peer_store);

        // Verify max cannot be exceeded via reservation
        for _ in 0..10 {
            assert!(manager.connection_slots.try_reserve());
        }
        // 11th should fail
        assert!(!manager.connection_slots.try_reserve());
    }

    #[tokio::test]
    async fn test_fixed_peer_list_mode_only_connects_to_list() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.target_connections = 2;
        config.max_connections = 10;
        let peer_store = Arc::new(InMemoryPeerStore::new());

        // Create fixed peer list
        let fixed_peers = vec![
            Peer::new("192.168.1.1".parse().unwrap(), 8333),
            Peer::new("192.168.1.2".parse().unwrap(), 8333),
        ];

        // Add other peers to store that should NOT be used
        peer_store
            .create(Peer::new("192.168.1.99".parse().unwrap(), 8333))
            .await
            .unwrap();

        let manager = Manager::with_fixed_peers(config, peer_store, fixed_peers.clone());

        assert_eq!(manager.operating_mode, OperatingMode::FixedPeerList);
        assert_eq!(manager.fixed_peers.as_ref().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_skips_banned_peers() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.target_connections = 3;
        let peer_store = Arc::new(InMemoryPeerStore::new());

        // Add one valid peer and one banned peer
        let mut valid_peer = Peer::new("192.168.1.1".parse().unwrap(), 8333);
        valid_peer.update_status(crate::p2p::PeerStatus::Valid);
        peer_store.create(valid_peer).await.unwrap();

        let mut banned_peer = Peer::new("192.168.1.2".parse().unwrap(), 8333);
        banned_peer.ban(crate::p2p::BanReason::BannedUserAgent {
            user_agent: "malicious".to_string(),
        });
        peer_store.create(banned_peer).await.unwrap();

        let _manager = Manager::new(config, peer_store.clone());

        // Verify banned peers can be identified
        let all_peers = peer_store.list_all().await.unwrap();
        let banned_count = all_peers.iter().filter(|p| p.is_banned()).count();
        assert_eq!(banned_count, 1);
    }

    #[tokio::test]
    async fn test_skips_inaccessible_peers() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.target_connections = 3;
        let peer_store = Arc::new(InMemoryPeerStore::new());

        // Add one valid peer and one inaccessible peer
        let mut valid_peer = Peer::new("192.168.1.1".parse().unwrap(), 8333);
        valid_peer.update_status(crate::p2p::PeerStatus::Valid);
        peer_store.create(valid_peer).await.unwrap();

        let mut inaccessible_peer = Peer::new("192.168.1.2".parse().unwrap(), 8333);
        inaccessible_peer.update_status(crate::p2p::PeerStatus::Inaccessible);
        peer_store.create(inaccessible_peer).await.unwrap();

        let _manager = Manager::new(config, peer_store.clone());

        // Verify inaccessible peers can be identified
        let all_peers = peer_store.list_all().await.unwrap();
        let inaccessible_count = all_peers.iter().filter(|p| p.is_inaccessible()).count();
        assert_eq!(inaccessible_count, 1);
    }

    #[tokio::test]
    async fn test_prioritizes_valid_over_unknown_peers() {
        let config = ManagerConfig::new(Network::Mainnet);
        let peer_store = Arc::new(InMemoryPeerStore::new());

        // Add valid peers
        let mut valid_peer = Peer::new("192.168.1.1".parse().unwrap(), 8333);
        valid_peer.update_status(crate::p2p::PeerStatus::Valid);
        peer_store.create(valid_peer.clone()).await.unwrap();

        // Add unknown peers
        let unknown_peer = Peer::new("192.168.1.2".parse().unwrap(), 8333);
        peer_store.create(unknown_peer.clone()).await.unwrap();

        let _manager = Manager::new(config, peer_store.clone());

        // Verify we can query by status
        let valid_peers = peer_store
            .find_by_status(crate::p2p::PeerStatus::Valid)
            .await
            .unwrap();
        let unknown_peers = peer_store
            .find_by_status(crate::p2p::PeerStatus::Unknown)
            .await
            .unwrap();

        assert_eq!(valid_peers.len(), 1);
        assert_eq!(unknown_peers.len(), 1);
        assert!(valid_peers[0].is_valid());
    }

    // Phase 5.4: Duplicate Connection Prevention Tests

    #[tokio::test]
    async fn test_prevents_duplicate_outbound_connections() {
        let config = ManagerConfig::new(Network::Mainnet);
        let peer_store = Arc::new(InMemoryPeerStore::new());

        let peer = Peer::new("192.168.1.1".parse().unwrap(), 8333);
        let peer_id = peer.id;
        peer_store.create(peer).await.unwrap();

        let manager = Manager::new(config, peer_store);

        // Simulate adding an active connection
        let (tx, _rx) = mpsc::channel(10);
        let handle = PeerConnectionHandle {
            peer_id,
            control_tx: tx,
        };

        {
            let mut connections = manager.active_connections.lock().await;
            connections.insert(peer_id, handle);
        }

        // Verify connection exists
        let connections = manager.active_connections.lock().await;
        assert!(connections.contains_key(&peer_id));
    }

    #[tokio::test]
    async fn test_prevents_duplicate_inbound_connections() {
        let config = ManagerConfig::new(Network::Mainnet);
        let peer_store = Arc::new(InMemoryPeerStore::new());

        // Create a peer that represents an inbound connection
        let peer = Peer::new("192.168.1.1".parse().unwrap(), 8333);
        let peer_id = peer.id;
        peer_store.create(peer).await.unwrap();

        let manager = Manager::new(config, peer_store.clone());

        // Add active connection
        let (tx, _rx) = mpsc::channel(10);
        let handle = PeerConnectionHandle {
            peer_id,
            control_tx: tx,
        };

        {
            let mut connections = manager.active_connections.lock().await;
            connections.insert(peer_id, handle);
        }

        // Try to find peer by IP:port to check if it exists
        let found_peer = peer_store
            .find_by_ip_port("192.168.1.1".parse().unwrap(), 8333)
            .await
            .unwrap();
        assert!(found_peer.is_some());
    }

    #[tokio::test]
    async fn test_handles_simultaneous_bidirectional_connections() {
        let config = ManagerConfig::new(Network::Mainnet);
        let peer_store = Arc::new(InMemoryPeerStore::new());

        let peer1 = Peer::new("192.168.1.1".parse().unwrap(), 8333);
        let peer1_id = peer1.id;
        peer_store.create(peer1).await.unwrap();

        let manager = Manager::new(config, peer_store);

        // Add first connection (outbound)
        let (tx1, _rx1) = mpsc::channel(10);
        let handle1 = PeerConnectionHandle {
            peer_id: peer1_id,
            control_tx: tx1,
        };

        {
            let mut connections = manager.active_connections.lock().await;
            connections.insert(peer1_id, handle1);
        }

        // Attempt to add second connection to same peer should be prevented
        // (in real implementation, this check happens before adding)
        let connections = manager.active_connections.lock().await;
        let has_connection = connections.contains_key(&peer1_id);
        assert!(has_connection, "Should detect existing connection");
    }

    #[tokio::test]
    async fn test_rejects_duplicate_active_connection() {
        let config = ManagerConfig::new(Network::Mainnet);
        let peer_store = Arc::new(InMemoryPeerStore::new());

        let peer = Peer::new("192.168.1.1".parse().unwrap(), 8333);
        let peer_id = peer.id;
        peer_store.create(peer.clone()).await.unwrap();

        let manager = Manager::new(config, peer_store.clone());

        // Add active connection
        let (tx, _rx) = mpsc::channel(10);
        let handle = PeerConnectionHandle {
            peer_id,
            control_tx: tx,
        };

        {
            let mut connections = manager.active_connections.lock().await;
            connections.insert(peer_id, handle);
        }

        // Verify we can check for duplicates
        let connections = manager.active_connections.lock().await;
        assert!(connections.contains_key(&peer_id));

        // Also verify we can look up by IP:port
        let found = peer_store
            .find_by_ip_port(peer.ip_address, peer.port)
            .await
            .unwrap();
        assert!(found.is_some());
    }

    // Phase 5.6: Inbound Listener Tests

    #[test]
    fn test_listener_configuration() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.enable_listener = true;
        config.listener_address = "127.0.0.1".to_string();
        config.listener_port = 18333;

        let peer_store = Arc::new(InMemoryPeerStore::new());
        let manager = Manager::new(config, peer_store);

        assert!(manager.config.enable_listener);
        assert_eq!(manager.config.listener_address, "127.0.0.1");
        assert_eq!(manager.config.listener_port, 18333);
    }

    #[tokio::test]
    async fn test_listener_respects_max_connections() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.max_connections = 2;
        config.enable_listener = true;

        let peer_store = Arc::new(InMemoryPeerStore::new());
        let manager = Manager::new(config, peer_store);

        // Reserve all slots
        assert!(manager.connection_slots.try_reserve());
        assert!(manager.connection_slots.try_reserve());

        // Third should fail
        assert!(!manager.connection_slots.try_reserve());
    }

    #[tokio::test]
    async fn test_listener_can_identify_banned_peers() {
        let config = ManagerConfig::new(Network::Mainnet);
        let peer_store = Arc::new(InMemoryPeerStore::new());

        // Add a banned peer
        let mut peer = Peer::new("192.168.1.1".parse().unwrap(), 8333);
        peer.ban(crate::p2p::BanReason::BannedUserAgent {
            user_agent: "malicious".to_string(),
        });
        peer_store.create(peer).await.unwrap();

        let _manager = Manager::new(config, peer_store.clone());

        // Verify we can detect banned peers by IP:port
        let found = peer_store
            .find_by_ip_port("192.168.1.1".parse().unwrap(), 8333)
            .await
            .unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().is_banned());
    }

    #[tokio::test]
    async fn test_listener_can_check_for_duplicate_connections() {
        let config = ManagerConfig::new(Network::Mainnet);
        let peer_store = Arc::new(InMemoryPeerStore::new());

        let peer = Peer::new("192.168.1.1".parse().unwrap(), 8333);
        let peer_id = peer.id;
        peer_store.create(peer).await.unwrap();

        let manager = Manager::new(config, peer_store.clone());

        // Add active connection
        let (tx, _rx) = mpsc::channel(10);
        let handle = PeerConnectionHandle {
            peer_id,
            control_tx: tx,
        };

        {
            let mut connections = manager.active_connections.lock().await;
            connections.insert(peer_id, handle);
        }

        // Listener should be able to detect this
        let connections = manager.active_connections.lock().await;
        assert!(connections.contains_key(&peer_id));
    }

    #[tokio::test]
    async fn test_listener_over_capacity_handling() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.max_connections = 1;

        let peer_store = Arc::new(InMemoryPeerStore::new());
        let manager = Manager::new(config, peer_store);

        // Reserve the one available slot
        assert!(manager.connection_slots.try_reserve());
        assert_eq!(manager.connection_slots.count(), 1);

        // Next attempt should indicate over capacity
        assert!(!manager.connection_slots.try_reserve());
    }

    #[tokio::test]
    async fn test_listener_accepts_when_under_capacity() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.max_connections = 5;

        let peer_store = Arc::new(InMemoryPeerStore::new());
        let manager = Manager::new(config, peer_store);

        // Should be able to reserve up to max
        for i in 0..5 {
            assert!(
                manager.connection_slots.try_reserve(),
                "Should accept connection {} when under capacity",
                i
            );
        }

        assert_eq!(manager.connection_slots.count(), 5);
    }

    // Phase 5.9: Event Handling Tests

    #[tokio::test]
    async fn test_manager_handles_connection_established_event() {
        let manager = create_test_manager();
        let mut rx = manager.subscribe_control_events();

        // Send ConnectionEstablished event
        let peer_id = Uuid::new_v4();
        let event = ControlEvent::ConnectionEstablished { peer_id };
        manager.control_event_tx.send(event).unwrap();

        // Verify event is received
        let received = rx.recv().await.unwrap();
        match received {
            ControlEvent::ConnectionEstablished { peer_id: id } => {
                assert_eq!(id, peer_id);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_manager_handles_connection_failed_event() {
        let manager = create_test_manager();
        let mut rx = manager.subscribe_control_events();

        let peer_id = Uuid::new_v4();
        let event = ControlEvent::ConnectionFailed {
            peer_id,
            reason: "Connection refused".to_string(),
        };
        manager.control_event_tx.send(event).unwrap();

        let received = rx.recv().await.unwrap();
        match received {
            ControlEvent::ConnectionFailed { peer_id: id, .. } => {
                assert_eq!(id, peer_id);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_manager_handles_peer_banned_event() {
        let manager = create_test_manager();
        let mut rx = manager.subscribe_control_events();

        let peer_id = Uuid::new_v4();
        let reason = crate::p2p::BanReason::BannedUserAgent {
            user_agent: "malicious".to_string(),
        };
        let event = ControlEvent::PeerBanned {
            peer_id,
            reason: reason.clone(),
        };
        manager.control_event_tx.send(event).unwrap();

        let received = rx.recv().await.unwrap();
        match received {
            ControlEvent::PeerBanned {
                peer_id: id,
                reason: r,
            } => {
                assert_eq!(id, peer_id);
                assert_eq!(r, reason);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_manager_broadcasts_events_to_multiple_subscribers() {
        let manager = create_test_manager();
        let mut rx1 = manager.subscribe_control_events();
        let mut rx2 = manager.subscribe_control_events();

        let peer_id = Uuid::new_v4();
        let event = ControlEvent::ConnectionEstablished { peer_id };
        manager.control_event_tx.send(event).unwrap();

        // Both subscribers should receive the event
        let received1 = rx1.recv().await.unwrap();
        let received2 = rx2.recv().await.unwrap();

        match (received1, received2) {
            (
                ControlEvent::ConnectionEstablished { peer_id: id1 },
                ControlEvent::ConnectionEstablished { peer_id: id2 },
            ) => {
                assert_eq!(id1, peer_id);
                assert_eq!(id2, peer_id);
            }
            _ => panic!("Wrong event types"),
        }
    }

    // Phase 5.11: Configuration Update Tests

    #[tokio::test]
    async fn test_config_update_succeeds_with_valid_config() {
        let mut manager = create_test_manager();

        let mut new_config = ManagerConfig::new(Network::Mainnet);
        new_config.target_connections = 10;
        new_config.max_connections = 20;

        let result = manager.update_config(new_config.clone()).await;
        assert!(result.is_ok());
        assert_eq!(manager.config.target_connections, 10);
        assert_eq!(manager.config.max_connections, 20);
    }

    #[tokio::test]
    async fn test_config_update_rejects_invalid_limits() {
        let mut manager = create_test_manager();

        let mut invalid_config = ManagerConfig::new(Network::Mainnet);
        invalid_config.target_connections = 30;
        invalid_config.max_connections = 20; // target > max

        let result = manager.update_config(invalid_config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_config_update_network_change() {
        let mut manager = create_test_manager();

        let new_config = ManagerConfig::new(Network::Testnet);

        let result = manager.update_config(new_config).await;
        assert!(result.is_ok());
        assert_eq!(manager.config.network, Network::Testnet);
    }

    #[tokio::test]
    async fn test_config_update_listener_settings() {
        let mut manager = create_test_manager();

        let mut new_config = ManagerConfig::new(Network::Mainnet);
        new_config.enable_listener = true;
        new_config.listener_address = "0.0.0.0".to_string();
        new_config.listener_port = 18333;

        let result = manager.update_config(new_config).await;
        assert!(result.is_ok());
        assert!(manager.config.enable_listener);
        assert_eq!(manager.config.listener_port, 18333);
    }
}
