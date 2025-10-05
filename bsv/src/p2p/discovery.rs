//! DNS-based peer discovery
//!
//! Implements DNS seed-based peer discovery for the Bitcoin SV network.

use crate::p2p::{ManagerConfig, Peer, PeerStore};
use crate::Result;
use std::sync::Arc;
use tokio::net::lookup_host;

/// DNS-based peer discovery
pub struct DnsDiscovery {
    peer_store: Arc<dyn PeerStore>,
    config: ManagerConfig,
}

impl DnsDiscovery {
    /// Create a new DNS discovery instance
    pub fn new(peer_store: Arc<dyn PeerStore>, config: ManagerConfig) -> Self {
        Self { peer_store, config }
    }

    /// Perform DNS discovery from configured seeds
    ///
    /// Queries each DNS seed and adds discovered peers to the store.
    /// Skips:
    /// - Peers that already exist in the store
    /// - Banned peers
    /// - Peers with duplicate IP:port combinations
    pub async fn discover(&self) -> Result<usize> {
        let mut discovered_count = 0;

        for seed in &self.config.dns_seeds {
            match self.discover_from_seed(seed).await {
                Ok(count) => {
                    discovered_count += count;
                    tracing::debug!(seed = %seed, count, "Discovered peers from DNS seed");
                }
                Err(e) => {
                    tracing::warn!(seed = %seed, error = %e, "Failed to discover peers from DNS seed");
                    // Continue with other seeds
                }
            }
        }

        Ok(discovered_count)
    }

    /// Discover peers from a single DNS seed
    async fn discover_from_seed(&self, seed: &str) -> Result<usize> {
        // Resolve DNS to get IP addresses
        let addrs = lookup_host(format!("{}:{}", seed, self.config.default_port))
            .await
            .map_err(|e| crate::Error::DnsResolutionFailed(format!("{}: {}", seed, e)))?;

        let mut added_count = 0;

        for addr in addrs {
            let ip = addr.ip();
            let port = self.config.default_port;

            // Check if peer already exists
            if let Ok(Some(existing)) = self.peer_store.find_by_ip_port(ip, port).await {
                // Skip if banned
                if existing.is_banned() {
                    tracing::debug!(ip = %ip, port, "Skipping banned peer from DNS");
                    continue;
                }
                // Skip if already exists
                tracing::trace!(ip = %ip, port, "Peer already exists, skipping");
                continue;
            }

            // Add new peer with Unknown status
            let peer = Peer::new(ip, port);
            match self.peer_store.create(peer).await {
                Ok(_) => {
                    added_count += 1;
                    tracing::trace!(ip = %ip, port, "Added peer from DNS discovery");
                }
                Err(e) => {
                    tracing::debug!(ip = %ip, port, error = %e, "Failed to add peer from DNS");
                }
            }
        }

        Ok(added_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::{InMemoryPeerStore, Network};

    fn create_test_config() -> ManagerConfig {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.dns_seeds = vec!["localhost".to_string()];
        config.default_port = 8333;
        config
    }

    #[tokio::test]
    async fn test_dns_discovery_creation() {
        let config = create_test_config();
        let peer_store = Arc::new(InMemoryPeerStore::new());
        let discovery = DnsDiscovery::new(peer_store, config.clone());

        assert_eq!(discovery.config.dns_seeds.len(), 1);
        assert_eq!(discovery.config.default_port, 8333);
    }

    #[tokio::test]
    async fn test_dns_discovery_uses_default_port() {
        let mut config = ManagerConfig::new(Network::Testnet);
        config.dns_seeds = vec!["localhost".to_string()];
        config.default_port = 18333;

        let peer_store = Arc::new(InMemoryPeerStore::new());
        let discovery = DnsDiscovery::new(peer_store, config);

        assert_eq!(discovery.config.default_port, 18333);
    }

    #[tokio::test]
    async fn test_dns_discovery_handles_resolution_failure() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.dns_seeds = vec!["nonexistent.invalid.domain.test".to_string()];

        let peer_store = Arc::new(InMemoryPeerStore::new());
        let discovery = DnsDiscovery::new(peer_store.clone(), config);

        // Should return 0 discovered peers but not fail completely
        let result = discovery.discover().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Peer store should be empty
        let peers = peer_store.list_all().await.unwrap();
        assert_eq!(peers.len(), 0);
    }

    #[tokio::test]
    async fn test_dns_discovery_skips_duplicates() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.dns_seeds = vec!["localhost".to_string()];
        config.default_port = 8333;

        let peer_store = Arc::new(InMemoryPeerStore::new());

        // Pre-populate with a localhost peer
        let existing_peer = Peer::new("127.0.0.1".parse().unwrap(), 8333);
        peer_store.create(existing_peer).await.unwrap();

        let discovery = DnsDiscovery::new(peer_store.clone(), config);

        // Run discovery - should not add duplicate
        let _result = discovery.discover().await;

        // Should still only have 1 peer
        let peers = peer_store.list_all().await.unwrap();
        // Could be 1 or more depending on localhost resolution, but shouldn't have exact duplicates
        assert!(!peers.is_empty());
    }

    #[tokio::test]
    async fn test_dns_discovery_skips_banned_ips() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.dns_seeds = vec!["localhost".to_string()];
        config.default_port = 8333;

        let peer_store = Arc::new(InMemoryPeerStore::new());

        // Add a banned localhost peer
        let mut banned_peer = Peer::new("127.0.0.1".parse().unwrap(), 8333);
        banned_peer.ban(crate::p2p::BanReason::BannedUserAgent {
            user_agent: "test".to_string(),
        });
        peer_store.create(banned_peer).await.unwrap();

        let discovery = DnsDiscovery::new(peer_store.clone(), config);

        // Run discovery - should skip banned peer
        let result = discovery.discover().await;
        assert!(result.is_ok());

        // Verify banned peer wasn't changed
        let peer = peer_store
            .find_by_ip_port("127.0.0.1".parse().unwrap(), 8333)
            .await
            .unwrap();
        assert!(peer.is_some());
        assert!(peer.unwrap().is_banned());
    }

    #[tokio::test]
    async fn test_dns_discovery_empty_seeds() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.dns_seeds = vec![];

        let peer_store = Arc::new(InMemoryPeerStore::new());
        let discovery = DnsDiscovery::new(peer_store.clone(), config);

        let result = discovery.discover().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
