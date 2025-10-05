// Peer storage and management
//
// This module defines the PeerStore trait for managing peers and provides
// an in-memory implementation with file persistence.

use crate::p2p::peer::{Peer, PeerStatus};
use crate::{Error, Result};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::fs;
use uuid::Uuid;

/// Trait for storing and managing peer data
///
/// All methods are async to support non-blocking I/O operations.
/// Implementations must be thread-safe.
#[async_trait::async_trait]
pub trait PeerStore: Send + Sync {
    /// Add a new peer to the store
    async fn create(&self, peer: Peer) -> Result<()>;

    /// Retrieve a peer by ID
    async fn read(&self, id: Uuid) -> Result<Peer>;

    /// Update an existing peer's information
    async fn update(&self, peer: Peer) -> Result<()>;

    /// Remove a peer from the store
    async fn delete(&self, id: Uuid) -> Result<()>;

    /// Retrieve all peers in the store
    async fn list_all(&self) -> Result<Vec<Peer>>;

    /// Find all peers with a specific status
    async fn find_by_status(&self, status: PeerStatus) -> Result<Vec<Peer>>;

    /// Find a peer by IP address and port
    async fn find_by_ip_port(&self, ip: IpAddr, port: u16) -> Result<Option<Peer>>;

    /// Count peers with a specific status
    async fn count_by_status(&self, status: PeerStatus) -> Result<usize>;
}

/// In-memory peer store with file persistence
///
/// This implementation stores peer data in memory during runtime and
/// persists to a JSON file on demand or periodically.
pub struct InMemoryPeerStore {
    peers: Arc<Mutex<HashMap<Uuid, Peer>>>,
    ip_index: Arc<Mutex<HashMap<(IpAddr, u16), Uuid>>>,
    file_path: Option<PathBuf>,
}

impl InMemoryPeerStore {
    /// Create a new in-memory peer store without file persistence
    pub fn new() -> Self {
        Self {
            peers: Arc::new(Mutex::new(HashMap::new())),
            ip_index: Arc::new(Mutex::new(HashMap::new())),
            file_path: None,
        }
    }

    /// Create a new in-memory peer store with file persistence
    pub fn with_file<P: AsRef<Path>>(path: P) -> Self {
        Self {
            peers: Arc::new(Mutex::new(HashMap::new())),
            ip_index: Arc::new(Mutex::new(HashMap::new())),
            file_path: Some(path.as_ref().to_path_buf()),
        }
    }

    /// Load peers from file if it exists
    pub async fn load_from_file(&self) -> Result<()> {
        let Some(ref path) = self.file_path else {
            return Ok(()); // No file configured
        };

        // Check if file exists
        if !fs::try_exists(path)
            .await
            .map_err(|e| Error::PeerStoreError(format!("Failed to check file existence: {}", e)))?
        {
            // File doesn't exist, that's OK
            return Ok(());
        }

        // Read file content
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| Error::PeerStoreError(format!("Failed to read peer store file: {}", e)))?;

        // Parse JSON
        let peers: Vec<Peer> = serde_json::from_str(&content).map_err(|e| {
            Error::PeerStoreError(format!("Failed to parse peer store file: {}", e))
        })?;

        // Load into store
        let mut peers_map = self.peers.lock().unwrap();
        let mut ip_index = self.ip_index.lock().unwrap();

        peers_map.clear();
        ip_index.clear();

        for peer in peers {
            ip_index.insert((peer.ip_address, peer.port), peer.id);
            peers_map.insert(peer.id, peer);
        }

        Ok(())
    }

    /// Save peers to file
    pub async fn save_to_file(&self) -> Result<()> {
        let Some(ref path) = self.file_path else {
            return Ok(()); // No file configured
        };

        // Get all peers
        let peers: Vec<Peer> = {
            let peers_map = self.peers.lock().unwrap();
            peers_map.values().cloned().collect()
        };

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&peers)
            .map_err(|e| Error::PeerStoreError(format!("Failed to serialize peers: {}", e)))?;

        // Write to file
        fs::write(path, json).await.map_err(|e| {
            Error::PeerStoreError(format!("Failed to write peer store file: {}", e))
        })?;

        Ok(())
    }

    /// Get the number of peers in the store
    pub fn len(&self) -> usize {
        self.peers.lock().unwrap().len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.peers.lock().unwrap().is_empty()
    }
}

impl Default for InMemoryPeerStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PeerStore for InMemoryPeerStore {
    async fn create(&self, peer: Peer) -> Result<()> {
        let mut peers = self.peers.lock().unwrap();
        let mut ip_index = self.ip_index.lock().unwrap();

        // Check for duplicate ID
        if peers.contains_key(&peer.id) {
            return Err(Error::DuplicatePeer);
        }

        // Check for duplicate IP:port
        if ip_index.contains_key(&(peer.ip_address, peer.port)) {
            return Err(Error::DuplicatePeer);
        }

        // Add to indexes
        ip_index.insert((peer.ip_address, peer.port), peer.id);
        peers.insert(peer.id, peer);

        Ok(())
    }

    async fn read(&self, id: Uuid) -> Result<Peer> {
        let peers = self.peers.lock().unwrap();
        peers.get(&id).cloned().ok_or(Error::PeerNotFound(id))
    }

    async fn update(&self, peer: Peer) -> Result<()> {
        let mut peers = self.peers.lock().unwrap();
        let mut ip_index = self.ip_index.lock().unwrap();

        // Check if peer exists
        let old_peer = peers.get(&peer.id).ok_or(Error::PeerNotFound(peer.id))?;

        // If IP:port changed, update index
        if old_peer.ip_address != peer.ip_address || old_peer.port != peer.port {
            // Remove old IP:port mapping
            ip_index.remove(&(old_peer.ip_address, old_peer.port));

            // Check if new IP:port is already used by another peer
            if let Some(&existing_id) = ip_index.get(&(peer.ip_address, peer.port)) {
                if existing_id != peer.id {
                    return Err(Error::DuplicatePeer);
                }
            }

            // Add new IP:port mapping
            ip_index.insert((peer.ip_address, peer.port), peer.id);
        }

        // Update peer
        peers.insert(peer.id, peer);

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let mut peers = self.peers.lock().unwrap();
        let mut ip_index = self.ip_index.lock().unwrap();

        let peer = peers.remove(&id).ok_or(Error::PeerNotFound(id))?;
        ip_index.remove(&(peer.ip_address, peer.port));

        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<Peer>> {
        let peers = self.peers.lock().unwrap();
        Ok(peers.values().cloned().collect())
    }

    async fn find_by_status(&self, status: PeerStatus) -> Result<Vec<Peer>> {
        let peers = self.peers.lock().unwrap();
        Ok(peers
            .values()
            .filter(|p| p.status == status)
            .cloned()
            .collect())
    }

    async fn find_by_ip_port(&self, ip: IpAddr, port: u16) -> Result<Option<Peer>> {
        let ip_index = self.ip_index.lock().unwrap();
        let peers = self.peers.lock().unwrap();

        if let Some(&id) = ip_index.get(&(ip, port)) {
            Ok(peers.get(&id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn count_by_status(&self, status: PeerStatus) -> Result<usize> {
        let peers = self.peers.lock().unwrap();
        Ok(peers.values().filter(|p| p.status == status).count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_create_peer() {
        let store = InMemoryPeerStore::new();
        let peer = Peer::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8333);

        store.create(peer.clone()).await.unwrap();

        let retrieved = store.read(peer.id).await.unwrap();
        assert_eq!(retrieved.id, peer.id);
        assert_eq!(retrieved.ip_address, peer.ip_address);
    }

    #[tokio::test]
    async fn test_create_duplicate_id_fails() {
        let store = InMemoryPeerStore::new();
        let peer = Peer::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8333);

        store.create(peer.clone()).await.unwrap();
        let result = store.create(peer).await;

        assert!(matches!(result, Err(Error::DuplicatePeer)));
    }

    #[tokio::test]
    async fn test_create_duplicate_ip_port_fails() {
        let store = InMemoryPeerStore::new();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        let peer1 = Peer::new(ip, 8333);
        let peer2 = Peer::new(ip, 8333);

        store.create(peer1).await.unwrap();
        let result = store.create(peer2).await;

        assert!(matches!(result, Err(Error::DuplicatePeer)));
    }

    #[tokio::test]
    async fn test_find_by_ip_port() {
        let store = InMemoryPeerStore::new();
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let peer = Peer::new(ip, 8333);

        store.create(peer.clone()).await.unwrap();

        let found = store.find_by_ip_port(ip, 8333).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, peer.id);

        let not_found = store.find_by_ip_port(ip, 9999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_status() {
        let store = InMemoryPeerStore::new();

        let mut peer1 = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);
        peer1.update_status(PeerStatus::Valid);

        let mut peer2 = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 8333);
        peer2.update_status(PeerStatus::Valid);

        let mut peer3 = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3)), 8333);
        peer3.update_status(PeerStatus::Banned);

        store.create(peer1).await.unwrap();
        store.create(peer2).await.unwrap();
        store.create(peer3).await.unwrap();

        let valid_peers = store.find_by_status(PeerStatus::Valid).await.unwrap();
        assert_eq!(valid_peers.len(), 2);

        let banned_peers = store.find_by_status(PeerStatus::Banned).await.unwrap();
        assert_eq!(banned_peers.len(), 1);
    }

    #[tokio::test]
    async fn test_count_by_status() {
        let store = InMemoryPeerStore::new();

        let mut peer1 = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);
        peer1.update_status(PeerStatus::Valid);

        let mut peer2 = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 8333);
        peer2.update_status(PeerStatus::Unknown);

        store.create(peer1).await.unwrap();
        store.create(peer2).await.unwrap();

        assert_eq!(store.count_by_status(PeerStatus::Valid).await.unwrap(), 1);
        assert_eq!(store.count_by_status(PeerStatus::Unknown).await.unwrap(), 1);
        assert_eq!(store.count_by_status(PeerStatus::Banned).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_update_peer() {
        let store = InMemoryPeerStore::new();
        let mut peer = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);

        store.create(peer.clone()).await.unwrap();

        peer.update_status(PeerStatus::Valid);
        store.update(peer.clone()).await.unwrap();

        let retrieved = store.read(peer.id).await.unwrap();
        assert_eq!(retrieved.status, PeerStatus::Valid);
    }

    #[tokio::test]
    async fn test_update_nonexistent_peer_fails() {
        let store = InMemoryPeerStore::new();
        let peer = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);

        let result = store.update(peer.clone()).await;
        assert!(matches!(result, Err(Error::PeerNotFound(_))));
    }

    #[tokio::test]
    async fn test_update_peer_ip_port() {
        let store = InMemoryPeerStore::new();
        let mut peer = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);

        store.create(peer.clone()).await.unwrap();

        // Change IP and port
        peer.ip_address = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
        peer.port = 9999;
        store.update(peer.clone()).await.unwrap();

        // Old IP:port should not find it
        let not_found = store
            .find_by_ip_port(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333)
            .await
            .unwrap();
        assert!(not_found.is_none());

        // New IP:port should find it
        let found = store
            .find_by_ip_port(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 9999)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, peer.id);
    }

    #[tokio::test]
    async fn test_delete_peer() {
        let store = InMemoryPeerStore::new();
        let peer = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);

        store.create(peer.clone()).await.unwrap();
        store.delete(peer.id).await.unwrap();

        let result = store.read(peer.id).await;
        assert!(matches!(result, Err(Error::PeerNotFound(_))));

        // IP index should also be cleaned up
        let not_found = store
            .find_by_ip_port(peer.ip_address, peer.port)
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_list_all() {
        let store = InMemoryPeerStore::new();

        let peer1 = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);
        let peer2 = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 8333);
        let peer3 = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3)), 8333);

        store.create(peer1).await.unwrap();
        store.create(peer2).await.unwrap();
        store.create(peer3).await.unwrap();

        let all_peers = store.list_all().await.unwrap();
        assert_eq!(all_peers.len(), 3);
    }

    #[tokio::test]
    async fn test_concurrent_updates() {
        let store = Arc::new(InMemoryPeerStore::new());

        // Create initial peer
        let peer = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);
        store.create(peer.clone()).await.unwrap();

        // Spawn multiple tasks that update the same peer
        let mut handles = vec![];
        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            let mut peer_clone = peer.clone();

            let handle = tokio::spawn(async move {
                peer_clone.update_status(if i % 2 == 0 {
                    PeerStatus::Valid
                } else {
                    PeerStatus::Unknown
                });
                store_clone.update(peer_clone).await
            });

            handles.push(handle);
        }

        // Wait for all updates to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Peer should still be in store
        let final_peer = store.read(peer.id).await.unwrap();
        assert!(final_peer.status == PeerStatus::Valid || final_peer.status == PeerStatus::Unknown);
    }

    #[tokio::test]
    async fn test_save_and_load_from_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Create store with some peers
        {
            let store = InMemoryPeerStore::with_file(&path);

            let mut peer1 = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);
            peer1.update_status(PeerStatus::Valid);

            let mut peer2 = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 8333);
            peer2.update_status(PeerStatus::Banned);

            store.create(peer1).await.unwrap();
            store.create(peer2).await.unwrap();

            // Save to file
            store.save_to_file().await.unwrap();
        }

        // Load from file in a new store
        {
            let store = InMemoryPeerStore::with_file(&path);
            store.load_from_file().await.unwrap();

            let all_peers = store.list_all().await.unwrap();
            assert_eq!(all_peers.len(), 2);

            let valid_peers = store.find_by_status(PeerStatus::Valid).await.unwrap();
            assert_eq!(valid_peers.len(), 1);

            let banned_peers = store.find_by_status(PeerStatus::Banned).await.unwrap();
            assert_eq!(banned_peers.len(), 1);
        }
    }

    #[tokio::test]
    async fn test_load_from_nonexistent_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        // Delete the file
        std::fs::remove_file(&path).unwrap();

        let store = InMemoryPeerStore::with_file(&path);

        // Should not error when file doesn't exist
        let result = store.load_from_file().await;
        assert!(result.is_ok());
        assert!(store.is_empty());
    }

    #[tokio::test]
    async fn test_save_without_file_path() {
        let store = InMemoryPeerStore::new();
        let peer = Peer::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8333);
        store.create(peer).await.unwrap();

        // Should not error when no file path configured
        let result = store.save_to_file().await;
        assert!(result.is_ok());
    }
}
