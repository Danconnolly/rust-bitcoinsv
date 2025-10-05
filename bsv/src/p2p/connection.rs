//! Peer connection state machine
//!
//! Manages the lifecycle of a single peer connection, including:
//! - Connection establishment
//! - Handshake
//! - Message exchange
//! - Reconnection and backoff
//! - Failure handling

use crate::p2p::{HandshakeState, Peer, PingPongState};
use crate::{Error, Result};
use std::time::Instant;

/// Connection state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// TCP connection in progress
    Connecting,
    /// TCP connected, waiting for handshake
    AwaitingHandshake,
    /// Handshake complete, connection active
    Connected,
    /// Connection rejected (e.g., over capacity)
    Rejected,
    /// Connection failed (terminal state)
    Failed,
}

/// Connection type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    /// We initiated the connection
    Outbound,
    /// Peer initiated the connection
    Inbound,
    /// Inbound but over capacity (will reject after handshake)
    OverCapacity,
}

/// Restart tracking for network failure recovery
#[derive(Debug, Clone)]
pub struct RestartTracking {
    /// Number of restarts in current window
    pub restart_count: usize,
    /// When the current restart window started
    pub restart_window_start: Option<Instant>,
    /// Maximum restarts allowed in window
    pub max_restarts: usize,
    /// Restart window duration (seconds)
    pub restart_window_secs: u64,
}

impl RestartTracking {
    pub fn new(max_restarts: usize, restart_window_secs: u64) -> Self {
        Self {
            restart_count: 0,
            restart_window_start: None,
            max_restarts,
            restart_window_secs,
        }
    }

    /// Check if restart is allowed
    pub fn can_restart(&self) -> bool {
        // If max_restarts is 0, never allow restart
        if self.max_restarts == 0 {
            return false;
        }

        if let Some(window_start) = self.restart_window_start {
            // Check if we're still in the window
            if window_start.elapsed().as_secs() < self.restart_window_secs {
                // Still in window, check count
                self.restart_count < self.max_restarts
            } else {
                // Window expired, restart is allowed
                true
            }
        } else {
            // No window yet, restart is allowed
            true
        }
    }

    /// Record a restart attempt
    pub fn record_restart(&mut self) {
        if let Some(window_start) = self.restart_window_start {
            // Check if window has expired
            if window_start.elapsed().as_secs() >= self.restart_window_secs {
                // Window expired, start new window
                self.restart_window_start = Some(Instant::now());
                self.restart_count = 1;
            } else {
                // Still in window, increment count
                self.restart_count += 1;
            }
        } else {
            // First restart, start window
            self.restart_window_start = Some(Instant::now());
            self.restart_count = 1;
        }
    }

    /// Reset restart tracking (called on successful connection)
    pub fn reset(&mut self) {
        self.restart_count = 0;
        self.restart_window_start = None;
    }
}

/// Peer connection
pub struct PeerConnection {
    pub peer: Peer,
    pub state: ConnectionState,
    pub connection_type: ConnectionType,
    pub handshake: HandshakeState,
    pub ping_pong: PingPongState,
    pub restart_tracking: RestartTracking,
    pub reject_after_handshake: bool,
    pub send_headers_mode: bool,
}

impl PeerConnection {
    /// Create a new outbound connection
    pub fn new_outbound(peer: Peer, restart_config: (usize, u64), ping_timeout: u64) -> Self {
        Self {
            peer,
            state: ConnectionState::Disconnected,
            connection_type: ConnectionType::Outbound,
            handshake: HandshakeState::new(),
            ping_pong: PingPongState::new(ping_timeout),
            restart_tracking: RestartTracking::new(restart_config.0, restart_config.1),
            reject_after_handshake: false,
            send_headers_mode: false,
        }
    }

    /// Create a new inbound connection
    pub fn new_inbound(peer: Peer, ping_timeout: u64) -> Self {
        Self {
            peer,
            state: ConnectionState::Disconnected,
            connection_type: ConnectionType::Inbound,
            handshake: HandshakeState::new(),
            ping_pong: PingPongState::new(ping_timeout),
            restart_tracking: RestartTracking::new(0, 0), // Inbound doesn't restart
            reject_after_handshake: false,
            send_headers_mode: false,
        }
    }

    /// Create an over-capacity connection (will reject after handshake)
    pub fn new_over_capacity(peer: Peer, ping_timeout: u64) -> Self {
        Self {
            peer,
            state: ConnectionState::Disconnected,
            connection_type: ConnectionType::OverCapacity,
            handshake: HandshakeState::new(),
            ping_pong: PingPongState::new(ping_timeout),
            restart_tracking: RestartTracking::new(0, 0),
            reject_after_handshake: true,
            send_headers_mode: false,
        }
    }

    /// Transition to connecting state
    pub fn transition_to_connecting(&mut self) -> Result<()> {
        match self.state {
            ConnectionState::Disconnected => {
                self.state = ConnectionState::Connecting;
                Ok(())
            }
            _ => Err(Error::Internal(format!(
                "Invalid state transition from {:?} to Connecting",
                self.state
            ))),
        }
    }

    /// Transition to awaiting handshake
    pub fn transition_to_awaiting_handshake(&mut self) -> Result<()> {
        match self.state {
            ConnectionState::Connecting => {
                self.state = ConnectionState::AwaitingHandshake;
                self.handshake.start(); // Start handshake timer
                Ok(())
            }
            _ => Err(Error::Internal(format!(
                "Invalid state transition from {:?} to AwaitingHandshake",
                self.state
            ))),
        }
    }

    /// Transition to connected
    pub fn transition_to_connected(&mut self) -> Result<()> {
        match self.state {
            ConnectionState::AwaitingHandshake => {
                if !self.handshake.is_complete() {
                    return Err(Error::HandshakeFailed("Handshake not complete".to_string()));
                }
                self.state = ConnectionState::Connected;
                self.restart_tracking.reset(); // Reset on successful connection
                Ok(())
            }
            _ => Err(Error::Internal(format!(
                "Invalid state transition from {:?} to Connected",
                self.state
            ))),
        }
    }

    /// Transition to rejected
    pub fn transition_to_rejected(&mut self) -> Result<()> {
        match self.state {
            ConnectionState::AwaitingHandshake | ConnectionState::Connected => {
                self.state = ConnectionState::Rejected;
                Ok(())
            }
            _ => Err(Error::Internal(format!(
                "Invalid state transition from {:?} to Rejected",
                self.state
            ))),
        }
    }

    /// Transition to failed
    pub fn transition_to_failed(&mut self) -> Result<()> {
        self.state = ConnectionState::Failed;
        Ok(())
    }

    /// Check if this error should trigger a restart
    pub fn should_restart(&self, error: &Error) -> bool {
        // Only outbound connections restart
        if self.connection_type != ConnectionType::Outbound {
            return false;
        }

        // Check if it's a network error (retryable)
        matches!(
            error,
            Error::ConnectionRefused | Error::ConnectionTimeout | Error::ConnectionReset
        )
    }

    /// Attempt to restart connection after network failure
    pub fn attempt_restart(&mut self) -> Result<()> {
        if !self.restart_tracking.can_restart() {
            return Err(Error::ConnectionFailed("Max restarts exceeded".to_string()));
        }

        // Record the restart
        self.restart_tracking.record_restart();

        // Reset to disconnected state for retry
        self.state = ConnectionState::Disconnected;
        self.handshake = HandshakeState::new();
        self.ping_pong.clear();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::VersionMessage;
    use std::net::IpAddr;
    use std::thread::sleep;
    use std::time::Duration;

    fn create_test_peer() -> Peer {
        Peer::new(IpAddr::V4([127, 0, 0, 1].into()), 8333)
    }

    #[test]
    fn test_outbound_connection_state_transitions() {
        let peer = create_test_peer();
        let mut conn = PeerConnection::new_outbound(peer, (3, 60), 30);

        // Initial state
        assert_eq!(conn.state, ConnectionState::Disconnected);
        assert_eq!(conn.connection_type, ConnectionType::Outbound);

        // Disconnected -> Connecting
        assert!(conn.transition_to_connecting().is_ok());
        assert_eq!(conn.state, ConnectionState::Connecting);

        // Connecting -> AwaitingHandshake
        assert!(conn.transition_to_awaiting_handshake().is_ok());
        assert_eq!(conn.state, ConnectionState::AwaitingHandshake);
        assert!(conn.handshake.started_at.is_some()); // Timer started

        // Complete handshake
        conn.handshake.mark_version_sent();
        conn.handshake.mark_version_received(create_test_version());
        conn.handshake.mark_verack_sent();
        conn.handshake.mark_verack_received();

        // AwaitingHandshake -> Connected
        assert!(conn.transition_to_connected().is_ok());
        assert_eq!(conn.state, ConnectionState::Connected);

        // Restart tracking should be reset
        assert_eq!(conn.restart_tracking.restart_count, 0);
    }

    #[test]
    fn test_inbound_connection_state_transitions() {
        let peer = create_test_peer();
        let mut conn = PeerConnection::new_inbound(peer, 30);

        assert_eq!(conn.connection_type, ConnectionType::Inbound);
        assert_eq!(conn.restart_tracking.max_restarts, 0); // Inbound doesn't restart

        // Same state flow as outbound
        conn.transition_to_connecting().unwrap();
        conn.transition_to_awaiting_handshake().unwrap();

        conn.handshake.mark_version_sent();
        conn.handshake.mark_version_received(create_test_version());
        conn.handshake.mark_verack_sent();
        conn.handshake.mark_verack_received();

        conn.transition_to_connected().unwrap();
        assert_eq!(conn.state, ConnectionState::Connected);
    }

    #[test]
    fn test_over_capacity_rejection_flow() {
        let peer = create_test_peer();
        let mut conn = PeerConnection::new_over_capacity(peer, 30);

        assert_eq!(conn.connection_type, ConnectionType::OverCapacity);
        assert!(conn.reject_after_handshake);

        // Go through handshake
        conn.transition_to_connecting().unwrap();
        conn.transition_to_awaiting_handshake().unwrap();

        conn.handshake.mark_version_sent();
        conn.handshake.mark_version_received(create_test_version());
        conn.handshake.mark_verack_sent();
        conn.handshake.mark_verack_received();

        // After handshake, should be rejected
        assert!(conn.transition_to_rejected().is_ok());
        assert_eq!(conn.state, ConnectionState::Rejected);
    }

    #[test]
    fn test_connection_restart_after_network_failure() {
        let peer = create_test_peer();
        let mut conn = PeerConnection::new_outbound(peer, (3, 60), 30);

        // Simulate connection attempt
        conn.transition_to_connecting().unwrap();

        // Network failure
        let error = Error::ConnectionTimeout;
        assert!(conn.should_restart(&error));

        // Attempt restart
        assert!(conn.attempt_restart().is_ok());
        assert_eq!(conn.state, ConnectionState::Disconnected);
        assert_eq!(conn.restart_tracking.restart_count, 1);
    }

    #[test]
    fn test_restart_limit_enforcement() {
        let peer = create_test_peer();
        let mut conn = PeerConnection::new_outbound(peer, (3, 60), 30); // Max 3 restarts

        // First 3 restarts should succeed
        for i in 1..=3 {
            assert!(conn.restart_tracking.can_restart());
            assert!(conn.attempt_restart().is_ok());
            assert_eq!(conn.restart_tracking.restart_count, i);
        }

        // 4th restart should fail
        assert!(!conn.restart_tracking.can_restart());
        let result = conn.attempt_restart();
        assert!(result.is_err());
        match result {
            Err(Error::ConnectionFailed(msg)) => {
                assert!(msg.contains("Max restarts"));
            }
            _ => panic!("Expected ConnectionFailed error"),
        }
    }

    #[test]
    fn test_inbound_does_not_reconnect() {
        let peer = create_test_peer();
        let conn = PeerConnection::new_inbound(peer, 30);

        // Network failure on inbound
        let error = Error::ConnectionReset;
        assert!(!conn.should_restart(&error)); // Inbound should NOT restart

        // Even if we try to restart, it should fail
        assert!(!conn.restart_tracking.can_restart());
    }

    #[test]
    fn test_restart_count_reset_after_window() {
        let peer = create_test_peer();
        let mut conn = PeerConnection::new_outbound(peer, (2, 1), 30); // 2 restarts, 1 sec window

        // First restart
        conn.attempt_restart().unwrap();
        assert_eq!(conn.restart_tracking.restart_count, 1);

        // Second restart
        conn.attempt_restart().unwrap();
        assert_eq!(conn.restart_tracking.restart_count, 2);

        // Wait for window to expire
        sleep(Duration::from_secs(2));

        // Next restart should start new window
        conn.attempt_restart().unwrap();
        assert_eq!(conn.restart_tracking.restart_count, 1); // Reset to 1
    }

    #[test]
    fn test_network_vs_non_network_error_handling() {
        let peer = create_test_peer();
        let conn = PeerConnection::new_outbound(peer, (3, 60), 30);

        // Network errors should trigger restart
        assert!(conn.should_restart(&Error::ConnectionTimeout));
        assert!(conn.should_restart(&Error::ConnectionRefused));
        assert!(conn.should_restart(&Error::ConnectionReset));

        // Non-network errors should NOT trigger restart
        assert!(!conn.should_restart(&Error::HandshakeTimeout));
        assert!(!conn.should_restart(&Error::HandshakeFailed("test".to_string())));
        assert!(!conn.should_restart(&Error::BannedUserAgent {
            user_agent: "test".to_string()
        }));
    }

    #[test]
    fn test_invalid_state_transitions() {
        let peer = create_test_peer();
        let mut conn = PeerConnection::new_outbound(peer, (3, 60), 30);

        // Can't go to awaiting_handshake from disconnected
        assert!(conn.transition_to_awaiting_handshake().is_err());

        // Can't go to connected from disconnected
        assert!(conn.transition_to_connected().is_err());

        // Move to connecting
        conn.transition_to_connecting().unwrap();

        // Can't go to connecting again
        assert!(conn.transition_to_connecting().is_err());
    }

    #[test]
    fn test_handshake_incomplete_prevents_connected() {
        let peer = create_test_peer();
        let mut conn = PeerConnection::new_outbound(peer, (3, 60), 30);

        conn.transition_to_connecting().unwrap();
        conn.transition_to_awaiting_handshake().unwrap();

        // Try to go to connected without completing handshake
        let result = conn.transition_to_connected();
        assert!(result.is_err());
        match result {
            Err(Error::HandshakeFailed(msg)) => {
                assert!(msg.contains("not complete"));
            }
            _ => panic!("Expected HandshakeFailed error"),
        }
    }

    fn create_test_version() -> VersionMessage {
        use crate::p2p::{NetworkAddress, Services, PROTOCOL_VERSION};
        VersionMessage {
            version: PROTOCOL_VERSION,
            services: Services::NETWORK,
            timestamp: 0,
            recv_addr: NetworkAddress {
                timestamp: None,
                services: Services::NETWORK,
                addr: IpAddr::V4([127, 0, 0, 1].into()),
                port: 8333,
            },
            from_addr: NetworkAddress {
                timestamp: None,
                services: Services::NETWORK,
                addr: IpAddr::V4([127, 0, 0, 1].into()),
                port: 8333,
            },
            nonce: 12345,
            user_agent: "/Bitcoin SV:1.0.0/".to_string(),
            start_height: 0,
            relay: true,
        }
    }
}
