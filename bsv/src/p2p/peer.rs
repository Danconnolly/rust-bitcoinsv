// Peer data structures and management
//
// This module defines the core Peer type and associated status tracking.

use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::time::SystemTime;
use uuid::Uuid;

/// Status of a peer connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    /// Peer is known to be accessible and working
    Valid,
    /// Peer cannot currently be reached (after max retries exceeded)
    Inaccessible,
    /// Peer is banned and should not be contacted
    Banned,
    /// Peer status has not yet been determined
    Unknown,
}

/// Reason for banning a peer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BanReason {
    /// Network mismatch (mainnet vs testnet vs regtest)
    NetworkMismatch { expected: String, received: String },
    /// Blockchain mismatch (not Bitcoin SV)
    BlockchainMismatch { received: String },
    /// User agent is banned
    BannedUserAgent { user_agent: String },
}

/// Represents a peer in the P2P network
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Peer {
    /// Unique identifier for the peer
    pub id: Uuid,
    /// IP address (IPv4 or IPv6)
    pub ip_address: IpAddr,
    /// Port number for connection
    pub port: u16,
    /// Current status of the peer
    pub status: PeerStatus,
    /// Timestamp when status was last updated
    #[serde(with = "system_time_serde")]
    pub status_timestamp: SystemTime,
    /// Optional ban reason (only set when status is Banned)
    pub ban_reason: Option<BanReason>,
}

impl Peer {
    /// Create a new peer with Unknown status
    pub fn new(ip_address: IpAddr, port: u16) -> Self {
        Self {
            id: Uuid::new_v4(),
            ip_address,
            port,
            status: PeerStatus::Unknown,
            status_timestamp: SystemTime::now(),
            ban_reason: None,
        }
    }

    /// Update peer status
    pub fn update_status(&mut self, status: PeerStatus) {
        self.status = status;
        self.status_timestamp = SystemTime::now();

        // Clear ban reason if no longer banned
        if status != PeerStatus::Banned {
            self.ban_reason = None;
        }
    }

    /// Ban this peer with a reason
    pub fn ban(&mut self, reason: BanReason) {
        self.status = PeerStatus::Banned;
        self.status_timestamp = SystemTime::now();
        self.ban_reason = Some(reason);
    }

    /// Check if peer is banned
    pub fn is_banned(&self) -> bool {
        self.status == PeerStatus::Banned
    }

    /// Check if peer is valid
    pub fn is_valid(&self) -> bool {
        self.status == PeerStatus::Valid
    }

    /// Check if peer is inaccessible
    pub fn is_inaccessible(&self) -> bool {
        self.status == PeerStatus::Inaccessible
    }
}

// Custom serde serialization for SystemTime
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_peer_creation_with_unknown_status() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let peer = Peer::new(ip, 8333);

        assert_eq!(peer.ip_address, ip);
        assert_eq!(peer.port, 8333);
        assert_eq!(peer.status, PeerStatus::Unknown);
        assert!(peer.ban_reason.is_none());
        assert!(!peer.is_banned());
        assert!(!peer.is_valid());
    }

    #[test]
    fn test_peer_status_transitions() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let mut peer = Peer::new(ip, 8333);

        // Unknown -> Valid
        peer.update_status(PeerStatus::Valid);
        assert_eq!(peer.status, PeerStatus::Valid);
        assert!(peer.is_valid());

        // Valid -> Inaccessible
        peer.update_status(PeerStatus::Inaccessible);
        assert_eq!(peer.status, PeerStatus::Inaccessible);
        assert!(peer.is_inaccessible());

        // Inaccessible -> Valid
        peer.update_status(PeerStatus::Valid);
        assert_eq!(peer.status, PeerStatus::Valid);
    }

    #[test]
    fn test_peer_ban_with_reason() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
        let mut peer = Peer::new(ip, 8333);

        let reason = BanReason::NetworkMismatch {
            expected: "mainnet".to_string(),
            received: "testnet".to_string(),
        };

        peer.ban(reason.clone());

        assert_eq!(peer.status, PeerStatus::Banned);
        assert!(peer.is_banned());
        assert_eq!(peer.ban_reason, Some(reason));
    }

    #[test]
    fn test_ban_reason_cleared_when_unbanned() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3));
        let mut peer = Peer::new(ip, 8333);

        // Ban the peer
        peer.ban(BanReason::BannedUserAgent {
            user_agent: "/badclient:1.0/".to_string(),
        });
        assert!(peer.ban_reason.is_some());

        // Unban by updating status
        peer.update_status(PeerStatus::Unknown);
        assert_eq!(peer.status, PeerStatus::Unknown);
        assert!(peer.ban_reason.is_none());
    }

    #[test]
    fn test_peer_serialization_deserialization() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        let mut peer = Peer::new(ip, 8333);
        peer.update_status(PeerStatus::Valid);

        // Serialize to JSON
        let json = serde_json::to_string(&peer).expect("Failed to serialize");

        // Deserialize back
        let deserialized: Peer = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(peer.id, deserialized.id);
        assert_eq!(peer.ip_address, deserialized.ip_address);
        assert_eq!(peer.port, deserialized.port);
        assert_eq!(peer.status, deserialized.status);
    }

    #[test]
    fn test_banned_peer_serialization_includes_reason() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 4));
        let mut peer = Peer::new(ip, 8333);

        peer.ban(BanReason::BlockchainMismatch {
            received: "Bitcoin Core".to_string(),
        });

        let json = serde_json::to_string(&peer).expect("Failed to serialize");
        let deserialized: Peer = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.status, PeerStatus::Banned);
        assert!(deserialized.ban_reason.is_some());
        assert_eq!(peer.ban_reason, deserialized.ban_reason);
    }

    #[test]
    fn test_ban_reason_variants() {
        let network_mismatch = BanReason::NetworkMismatch {
            expected: "mainnet".to_string(),
            received: "testnet".to_string(),
        };

        let blockchain_mismatch = BanReason::BlockchainMismatch {
            received: "Bitcoin ABC".to_string(),
        };

        let banned_user_agent = BanReason::BannedUserAgent {
            user_agent: "/malicious:0.1/".to_string(),
        };

        // Verify all variants are distinct
        assert_ne!(
            serde_json::to_string(&network_mismatch).unwrap(),
            serde_json::to_string(&blockchain_mismatch).unwrap()
        );
        assert_ne!(
            serde_json::to_string(&blockchain_mismatch).unwrap(),
            serde_json::to_string(&banned_user_agent).unwrap()
        );
    }

    #[test]
    fn test_peer_status_timestamp_updates() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));
        let mut peer = Peer::new(ip, 8333);

        let initial_timestamp = peer.status_timestamp;

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        peer.update_status(PeerStatus::Valid);

        // Timestamp should have been updated
        assert!(peer.status_timestamp > initial_timestamp);
    }

    #[test]
    fn test_multiple_ban_reason_updates() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 6));
        let mut peer = Peer::new(ip, 8333);

        // First ban
        peer.ban(BanReason::BannedUserAgent {
            user_agent: "/bad1:1.0/".to_string(),
        });

        let first_reason = peer.ban_reason.clone();

        // Second ban with different reason
        peer.ban(BanReason::NetworkMismatch {
            expected: "mainnet".to_string(),
            received: "testnet".to_string(),
        });

        // Reason should be updated
        assert_ne!(peer.ban_reason, first_reason);
        assert!(peer.is_banned());
    }
}
