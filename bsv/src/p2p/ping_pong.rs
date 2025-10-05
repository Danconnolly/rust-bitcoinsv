//! Ping/Pong keep-alive protocol
//!
//! The ping/pong protocol is used to:
//! - Keep connections alive
//! - Detect dead connections
//! - Measure round-trip time
//!
//! Protocol:
//! 1. Send Ping with random nonce
//! 2. Track sent nonce with timestamp
//! 3. Expect Pong with same nonce
//! 4. Validate received Pong nonce matches sent Ping
//! 5. Respond to received Ping with Pong echoing the nonce

use crate::{Error, Result};
use std::collections::HashMap;
use std::time::Instant;

/// Ping/Pong state tracker
#[derive(Debug)]
pub struct PingPongState {
    /// Map of sent ping nonces to when they were sent
    sent_pings: HashMap<u64, Instant>,
    /// Timeout duration for ping responses (seconds)
    ping_timeout_secs: u64,
}

impl PingPongState {
    /// Create a new ping/pong state tracker
    pub fn new(ping_timeout_secs: u64) -> Self {
        Self {
            sent_pings: HashMap::new(),
            ping_timeout_secs,
        }
    }

    /// Generate a new ping nonce
    pub fn generate_nonce() -> u64 {
        rand::random()
    }

    /// Record a sent ping
    pub fn record_ping(&mut self, nonce: u64) {
        self.sent_pings.insert(nonce, Instant::now());
    }

    /// Validate a received pong against sent pings
    /// Returns Ok(round_trip_time) if valid, Err if invalid nonce
    pub fn validate_pong(&mut self, nonce: u64) -> Result<std::time::Duration> {
        match self.sent_pings.remove(&nonce) {
            Some(sent_at) => {
                let rtt = sent_at.elapsed();
                Ok(rtt)
            }
            None => Err(Error::HandshakeFailed(format!(
                "Received pong with unknown nonce: {}",
                nonce
            ))),
        }
    }

    /// Check for timed out pings
    /// Returns Vec of timed out nonces
    pub fn check_timeouts(&mut self) -> Vec<u64> {
        let timeout_duration = std::time::Duration::from_secs(self.ping_timeout_secs);
        let mut timed_out = Vec::new();

        self.sent_pings.retain(|&nonce, &mut sent_at| {
            if sent_at.elapsed() > timeout_duration {
                timed_out.push(nonce);
                false // remove from map
            } else {
                true // keep in map
            }
        });

        timed_out
    }

    /// Get number of pending pings
    pub fn pending_count(&self) -> usize {
        self.sent_pings.len()
    }

    /// Clear all pending pings
    pub fn clear(&mut self) {
        self.sent_pings.clear();
    }
}

/// Create a pong response nonce (echoes the ping nonce)
pub fn create_pong_nonce(ping_nonce: u64) -> u64 {
    ping_nonce
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_ping_nonce_generation() {
        let nonce1 = PingPongState::generate_nonce();
        let nonce2 = PingPongState::generate_nonce();

        // Nonces should be random (different)
        assert_ne!(nonce1, nonce2, "Nonces should be different");

        // Nonces should be non-zero (very high probability)
        assert_ne!(nonce1, 0, "Nonce should be non-zero");
        assert_ne!(nonce2, 0, "Nonce should be non-zero");
    }

    #[test]
    fn test_ping_nonce_tracking() {
        let mut state = PingPongState::new(5);

        // Record some pings
        let nonce1 = 12345u64;
        let nonce2 = 67890u64;

        state.record_ping(nonce1);
        assert_eq!(state.pending_count(), 1);

        state.record_ping(nonce2);
        assert_eq!(state.pending_count(), 2);

        // Validate we can retrieve them
        assert!(state.sent_pings.contains_key(&nonce1));
        assert!(state.sent_pings.contains_key(&nonce2));
    }

    #[test]
    fn test_pong_validation_matches_nonce() {
        let mut state = PingPongState::new(5);

        let nonce = 123456789u64;
        state.record_ping(nonce);

        // Valid pong with matching nonce
        let result = state.validate_pong(nonce);
        assert!(result.is_ok(), "Should accept valid pong nonce");

        let rtt = result.unwrap();
        assert!(rtt.as_micros() > 0, "RTT should be positive");

        // Nonce should be removed after validation
        assert_eq!(state.pending_count(), 0);

        // Invalid pong with unknown nonce
        let result = state.validate_pong(999999u64);
        assert!(result.is_err(), "Should reject unknown pong nonce");

        match result {
            Err(Error::HandshakeFailed(msg)) => {
                assert!(msg.contains("unknown nonce"));
            }
            _ => panic!("Expected HandshakeFailed error"),
        }
    }

    #[test]
    fn test_ping_timeout() {
        let mut state = PingPongState::new(0); // 0 second timeout for testing

        let nonce1 = 111u64;
        let nonce2 = 222u64;

        state.record_ping(nonce1);
        sleep(Duration::from_millis(10)); // Wait a bit
        state.record_ping(nonce2);

        // Check for timeouts
        let timed_out = state.check_timeouts();

        // Both should time out since timeout is 0 seconds
        assert_eq!(timed_out.len(), 2, "Both pings should time out");
        assert!(timed_out.contains(&nonce1));
        assert!(timed_out.contains(&nonce2));

        // Should be removed from tracking
        assert_eq!(state.pending_count(), 0);
    }

    #[test]
    fn test_partial_timeout() {
        let mut state = PingPongState::new(1); // 1 second timeout

        let nonce1 = 111u64;
        let nonce2 = 222u64;

        state.record_ping(nonce1);
        sleep(Duration::from_millis(1100)); // Wait past timeout
        state.record_ping(nonce2); // This one won't timeout yet

        let timed_out = state.check_timeouts();

        // Only first should time out
        assert_eq!(timed_out.len(), 1, "Only first ping should time out");
        assert_eq!(timed_out[0], nonce1);

        // Second should still be tracked
        assert_eq!(state.pending_count(), 1);
        assert!(state.sent_pings.contains_key(&nonce2));
    }

    #[test]
    fn test_respond_to_received_ping() {
        // When we receive a ping with nonce N, we respond with pong with same nonce N
        let ping_nonce = 0xABCDEF123456u64;
        let pong_nonce = create_pong_nonce(ping_nonce);

        assert_eq!(
            ping_nonce, pong_nonce,
            "Pong should echo ping nonce exactly"
        );
    }

    #[test]
    fn test_clear_pending_pings() {
        let mut state = PingPongState::new(5);

        state.record_ping(111);
        state.record_ping(222);
        state.record_ping(333);

        assert_eq!(state.pending_count(), 3);

        state.clear();

        assert_eq!(state.pending_count(), 0);
        assert!(state.sent_pings.is_empty());
    }

    #[test]
    fn test_round_trip_time_measurement() {
        let mut state = PingPongState::new(5);

        let nonce = 12345u64;
        state.record_ping(nonce);

        // Wait a bit to simulate network delay
        sleep(Duration::from_millis(50));

        // Validate pong and check RTT
        let result = state.validate_pong(nonce);
        assert!(result.is_ok());

        let rtt = result.unwrap();
        assert!(
            rtt.as_millis() >= 50,
            "RTT should be at least 50ms, got {}ms",
            rtt.as_millis()
        );
    }

    #[test]
    fn test_duplicate_pong_rejected() {
        let mut state = PingPongState::new(5);

        let nonce = 55555u64;
        state.record_ping(nonce);

        // First pong succeeds
        let result1 = state.validate_pong(nonce);
        assert!(result1.is_ok());

        // Second pong with same nonce fails (already consumed)
        let result2 = state.validate_pong(nonce);
        assert!(result2.is_err());
    }

    #[test]
    fn test_multiple_concurrent_pings() {
        let mut state = PingPongState::new(5);

        let nonces: Vec<u64> = (1..=10).collect();

        // Send multiple pings
        for &nonce in &nonces {
            state.record_ping(nonce);
        }

        assert_eq!(state.pending_count(), 10);

        // Receive pongs in different order
        assert!(state.validate_pong(5).is_ok());
        assert!(state.validate_pong(1).is_ok());
        assert!(state.validate_pong(9).is_ok());

        assert_eq!(state.pending_count(), 7);

        // Remaining nonces still tracked
        assert!(state.sent_pings.contains_key(&2));
        assert!(state.sent_pings.contains_key(&10));
    }
}
