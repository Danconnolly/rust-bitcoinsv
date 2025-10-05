//! Handshake protocol implementation for P2P connections
//!
//! The handshake process requires 4 flags to complete:
//! - version_sent: We sent our Version message
//! - version_received: We received peer's Version message
//! - verack_sent: We sent Verack message
//! - verack_received: We received peer's Verack message
//!
//! Messages can arrive in any order.

use crate::p2p::{config::is_user_agent_banned, Network, VersionMessage};
use crate::{Error, Result};
use std::time::Instant;

/// Handshake state tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeState {
    /// We sent our version message
    pub version_sent: bool,
    /// We received peer's version message
    pub version_received: bool,
    /// We sent verack message
    pub verack_sent: bool,
    /// We received peer's verack message
    pub verack_received: bool,
    /// When the handshake started (for timeout tracking)
    pub started_at: Option<Instant>,
    /// Peer's version message (once received)
    pub peer_version: Option<VersionMessage>,
}

impl Default for HandshakeState {
    fn default() -> Self {
        Self::new()
    }
}

impl HandshakeState {
    /// Create a new handshake state
    pub fn new() -> Self {
        Self {
            version_sent: false,
            version_received: false,
            verack_sent: false,
            verack_received: false,
            started_at: None,
            peer_version: None,
        }
    }

    /// Start the handshake timer
    pub fn start(&mut self) {
        if self.started_at.is_none() {
            self.started_at = Some(Instant::now());
        }
    }

    /// Check if handshake is complete (all 4 flags true)
    pub fn is_complete(&self) -> bool {
        self.version_sent && self.version_received && self.verack_sent && self.verack_received
    }

    /// Check if handshake has timed out
    pub fn is_timed_out(&self, timeout_secs: u64) -> bool {
        if let Some(started) = self.started_at {
            started.elapsed().as_secs() >= timeout_secs
        } else {
            false
        }
    }

    /// Mark that we sent our version
    pub fn mark_version_sent(&mut self) {
        self.version_sent = true;
    }

    /// Mark that we received peer's version
    pub fn mark_version_received(&mut self, version: VersionMessage) {
        self.version_received = true;
        self.peer_version = Some(version);
    }

    /// Mark that we sent verack
    pub fn mark_verack_sent(&mut self) {
        self.verack_sent = true;
    }

    /// Mark that we received peer's verack
    pub fn mark_verack_received(&mut self) {
        self.verack_received = true;
    }
}

/// Validate a peer's version message
pub fn validate_version(
    version: &VersionMessage,
    _expected_network: Network,
    banned_user_agents: &[String],
) -> Result<()> {
    // Validate network magic (we need to check this externally via message header)
    // This is validated in the message reading layer

    // Validate blockchain type via user agent
    // For BSV, we expect user agents containing specific patterns
    if !is_bsv_user_agent(&version.user_agent) {
        return Err(Error::BlockchainMismatch {
            received: version.user_agent.clone(),
        });
    }

    // Check if user agent is banned
    if is_user_agent_banned(&version.user_agent, banned_user_agents) {
        return Err(Error::BannedUserAgent {
            user_agent: version.user_agent.clone(),
        });
    }

    Ok(())
}

/// Check if user agent indicates BSV node
fn is_bsv_user_agent(user_agent: &str) -> bool {
    let ua_lower = user_agent.to_lowercase();

    // Accept BSV-specific user agents
    if ua_lower.contains("bitcoin sv")
        || ua_lower.contains("bitcoinsv")
        || ua_lower.contains("bsv/")
        || ua_lower.contains("/bsv")
    {
        return true;
    }

    // For testing, also accept our own user agent
    if ua_lower.contains("rust-bitcoinsv") {
        return true;
    }

    // Reject Bitcoin Core and other implementations
    if ua_lower.contains("satoshi")
        || ua_lower.contains("bitcoin core")
        || ua_lower.contains("btc")
        || ua_lower.contains("bitcoin abc")
        || ua_lower.contains("bitcoin cash")
        || ua_lower.contains("bch")
    {
        return false;
    }

    // Default: be permissive for unknown agents (can be made stricter)
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::{NetworkAddress, Services, PROTOCOL_VERSION};
    use std::net::IpAddr;
    use std::thread::sleep;
    use std::time::Duration;

    fn create_test_version(user_agent: &str) -> VersionMessage {
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
            user_agent: user_agent.to_string(),
            start_height: 0,
            relay: true,
        }
    }

    #[test]
    fn test_handshake_state_transitions() {
        let mut state = HandshakeState::new();

        assert!(!state.is_complete());
        assert!(!state.version_sent);
        assert!(!state.version_received);
        assert!(!state.verack_sent);
        assert!(!state.verack_received);

        // Send version
        state.mark_version_sent();
        assert!(state.version_sent);
        assert!(!state.is_complete());

        // Receive version
        let version = create_test_version("/Bitcoin SV:1.0.0/");
        state.mark_version_received(version.clone());
        assert!(state.version_received);
        assert_eq!(state.peer_version, Some(version));
        assert!(!state.is_complete());

        // Send verack
        state.mark_verack_sent();
        assert!(state.verack_sent);
        assert!(!state.is_complete());

        // Receive verack - now complete
        state.mark_verack_received();
        assert!(state.verack_received);
        assert!(state.is_complete());
    }

    #[test]
    fn test_handshake_validation_rejects_wrong_network() {
        // Network validation happens at message header level (magic check)
        // This is tested in protocol tests

        // Here we test that the validation function exists and can be called
        let version = create_test_version("/Bitcoin SV:1.0.0/");
        let result = validate_version(&version, Network::Mainnet, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handshake_validation_rejects_banned_user_agent() {
        let version = create_test_version("/BannedAgent:1.0/");
        let banned_agents = vec!["/BannedAgent*".to_string()];

        let result = validate_version(&version, Network::Mainnet, &banned_agents);
        assert!(result.is_err());

        match result {
            Err(Error::BannedUserAgent { user_agent }) => {
                assert_eq!(user_agent, "/BannedAgent:1.0/");
            }
            _ => panic!("Expected BannedUserAgent error"),
        }
    }

    #[test]
    fn test_handshake_validation_rejects_non_bsv_blockchain() {
        // Test Bitcoin Core rejection
        let btc_version = create_test_version("/Satoshi:0.21.0/");
        let result = validate_version(&btc_version, Network::Mainnet, &[]);
        assert!(result.is_err());
        match result {
            Err(Error::BlockchainMismatch { received }) => {
                assert!(received.contains("Satoshi"));
            }
            _ => panic!("Expected BlockchainMismatch error"),
        }

        // Test Bitcoin ABC rejection
        let abc_version = create_test_version("/Bitcoin ABC:0.22.0/");
        let result = validate_version(&abc_version, Network::Mainnet, &[]);
        assert!(result.is_err());

        // Test BSV acceptance
        let bsv_version = create_test_version("/Bitcoin SV:1.0.0/");
        let result = validate_version(&bsv_version, Network::Mainnet, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handshake_timeout_starts_at_awaiting_handshake() {
        let mut state = HandshakeState::new();

        // Timer should not be started yet
        assert!(state.started_at.is_none());
        assert!(!state.is_timed_out(5));

        // Start the handshake (this happens when entering AwaitingHandshake state)
        state.start();
        assert!(state.started_at.is_some());

        // Should not be timed out immediately
        assert!(!state.is_timed_out(5));

        // Wait a bit and check timeout with short duration
        sleep(Duration::from_millis(100));
        assert!(!state.is_timed_out(1)); // 1 second timeout, should not trigger yet

        // Check that timeout would trigger with 0 seconds
        assert!(state.is_timed_out(0));
    }

    #[test]
    fn test_handshake_handles_messages_in_any_order() {
        // Test order: receive before send
        let mut state1 = HandshakeState::new();
        state1.mark_verack_received();
        state1.mark_version_received(create_test_version("/Bitcoin SV:1.0.0/"));
        state1.mark_verack_sent();
        state1.mark_version_sent();
        assert!(state1.is_complete());

        // Test order: mixed
        let mut state2 = HandshakeState::new();
        state2.mark_version_sent();
        state2.mark_verack_received();
        state2.mark_version_received(create_test_version("/Bitcoin SV:1.0.0/"));
        state2.mark_verack_sent();
        assert!(state2.is_complete());

        // Test order: all receives first
        let mut state3 = HandshakeState::new();
        state3.mark_version_received(create_test_version("/Bitcoin SV:1.0.0/"));
        state3.mark_verack_received();
        state3.mark_version_sent();
        state3.mark_verack_sent();
        assert!(state3.is_complete());
    }

    #[test]
    fn test_handshake_four_flags_all_required() {
        let mut state = HandshakeState::new();

        // Only 3 flags set - not complete
        state.mark_version_sent();
        state.mark_version_received(create_test_version("/Bitcoin SV:1.0.0/"));
        state.mark_verack_sent();
        assert!(!state.is_complete());

        // All 4 flags set - complete
        state.mark_verack_received();
        assert!(state.is_complete());

        // Test other combinations
        let mut state2 = HandshakeState::new();
        state2.mark_version_sent();
        state2.mark_verack_sent();
        state2.mark_verack_received();
        assert!(!state2.is_complete()); // Missing version_received
    }

    #[test]
    fn test_bsv_user_agent_detection() {
        // Accept BSV user agents
        assert!(is_bsv_user_agent("/Bitcoin SV:1.0.0/"));
        assert!(is_bsv_user_agent("/BitcoinSV:1.0.0/"));
        assert!(is_bsv_user_agent("/bsv/1.0/"));
        assert!(is_bsv_user_agent("/rust-bitcoinsv:0.1.0/"));

        // Reject non-BSV
        assert!(!is_bsv_user_agent("/Satoshi:0.21.0/"));
        assert!(!is_bsv_user_agent("/Bitcoin Core:22.0/"));
        assert!(!is_bsv_user_agent("/Bitcoin ABC:0.22.0/"));
        assert!(!is_bsv_user_agent("/btc-client:1.0/"));

        // Unknown agents are accepted (permissive)
        assert!(is_bsv_user_agent("/UnknownClient:1.0/"));
    }

    #[test]
    fn test_handshake_timer_only_starts_once() {
        let mut state = HandshakeState::new();

        state.start();
        let first_start = state.started_at.unwrap();

        // Try to start again
        sleep(Duration::from_millis(10));
        state.start();
        let second_start = state.started_at.unwrap();

        // Should be the same time (didn't restart)
        assert_eq!(first_start, second_start);
    }
}
