// P2P module configuration
//
// This module defines configuration structures for the P2P Manager and connections.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Bitcoin network type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    /// Bitcoin SV mainnet
    Mainnet,
    /// Bitcoin SV testnet
    Testnet,
    /// Bitcoin SV regression test network
    Regtest,
}

impl Network {
    /// Get the magic value for this network
    pub fn magic(&self) -> u32 {
        match self {
            Network::Mainnet => crate::p2p::MAGIC_MAINNET,
            Network::Testnet => crate::p2p::MAGIC_TESTNET,
            Network::Regtest => crate::p2p::MAGIC_REGTEST,
        }
    }

    /// Get the default port for this network
    pub fn default_port(&self) -> u16 {
        match self {
            Network::Mainnet => 8333,
            Network::Testnet => 18333,
            Network::Regtest => 18444,
        }
    }
}

impl std::str::FromStr for Network {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "mainnet" => Ok(Network::Mainnet),
            "testnet" => Ok(Network::Testnet),
            "regtest" => Ok(Network::Regtest),
            _ => Err(Error::InvalidConfiguration(format!(
                "Invalid network: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Regtest => write!(f, "regtest"),
        }
    }
}

/// Manager-level configuration
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// Network type
    pub network: Network,

    /// Target number of total connections to maintain
    pub target_connections: usize,

    /// Maximum number of total connections allowed
    pub max_connections: usize,

    /// DNS seeds for peer discovery
    pub dns_seeds: Vec<String>,

    /// Default port for discovered peers
    pub default_port: u16,

    /// File path for persisting peer data
    pub peer_store_file_path: Option<String>,

    /// Enable inbound connection listener
    pub enable_listener: bool,

    /// IP address to bind listener to
    pub listener_address: String,

    /// Port to bind listener to
    pub listener_port: u16,

    /// User agent string patterns to ban (supports wildcards or contains matching)
    pub banned_user_agents: Vec<String>,

    /// Minimum log level
    pub log_level: String,

    /// Enable OpenTelemetry
    pub enable_telemetry: bool,

    /// OTLP endpoint URL (optional)
    pub telemetry_endpoint: Option<String>,

    /// Service name for telemetry
    pub service_name: String,
}

impl ManagerConfig {
    /// Create a new ManagerConfig with required parameters
    pub fn new(network: Network) -> Self {
        Self {
            network,
            target_connections: 8,
            max_connections: 20,
            dns_seeds: Self::default_dns_seeds(network),
            default_port: network.default_port(),
            peer_store_file_path: None,
            enable_listener: false,
            listener_address: "0.0.0.0".to_string(),
            listener_port: network.default_port(),
            banned_user_agents: Vec::new(),
            log_level: "info".to_string(),
            enable_telemetry: false,
            telemetry_endpoint: None,
            service_name: "bsv-p2p-manager".to_string(),
        }
    }

    /// Get default DNS seeds for a network
    fn default_dns_seeds(network: Network) -> Vec<String> {
        match network {
            Network::Mainnet => vec![
                "seed.bitcoinsv.io".to_string(),
                "seed.cascharia.com".to_string(),
                "seed.satoshisvision.network".to_string(),
            ],
            Network::Testnet => vec!["testnet-seed.bitcoinsv.io".to_string()],
            Network::Regtest => vec![], // No DNS seeds for regtest
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate connection limits
        if self.target_connections > self.max_connections {
            return Err(Error::InvalidConnectionLimits {
                target: self.target_connections,
                max: self.max_connections,
            });
        }

        // Target connections must be at least 1 in normal mode
        if self.target_connections == 0 {
            return Err(Error::InvalidConfiguration(
                "target_connections must be at least 1".to_string(),
            ));
        }

        // Max connections must be at least 1
        if self.max_connections == 0 {
            return Err(Error::InvalidConfiguration(
                "max_connections must be at least 1".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self::new(Network::Mainnet)
    }
}

/// Connection-level configuration
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Duration between outbound Ping messages
    pub ping_interval: Duration,

    /// Maximum time to wait for Pong response
    pub ping_timeout: Duration,

    /// Maximum time to wait for handshake completion
    pub handshake_timeout: Duration,

    /// Starting backoff delay for reconnection attempts
    pub initial_backoff: Duration,

    /// Maximum number of connection retry attempts
    pub max_retries: usize,

    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,

    /// Maximum automatic restarts for network failures
    pub max_restarts: usize,

    /// Time window for counting restarts
    pub restart_window: Duration,
}

impl ConnectionConfig {
    /// Create a new ConnectionConfig with default values
    pub fn new() -> Self {
        Self {
            ping_interval: Duration::from_secs(5 * 60),      // 5 minutes
            ping_timeout: Duration::from_secs(30),            // 30 seconds
            handshake_timeout: Duration::from_secs(10),       // 10 seconds
            initial_backoff: Duration::from_secs(5),          // 5 seconds
            max_retries: 10,
            backoff_multiplier: 2.0,
            max_restarts: 3,
            restart_window: Duration::from_secs(60 * 60),    // 1 hour
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.backoff_multiplier <= 1.0 {
            return Err(Error::InvalidConfiguration(
                "backoff_multiplier must be greater than 1.0".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to check if a user agent matches a banned pattern
pub fn is_user_agent_banned(user_agent: &str, banned_patterns: &[String]) -> bool {
    for pattern in banned_patterns {
        if pattern.contains('*') {
            // Simple wildcard matching
            if wildcard_match(pattern, user_agent) {
                return true;
            }
        } else {
            // Simple contains matching
            if user_agent.contains(pattern) {
                return true;
            }
        }
    }
    false
}

/// Simple wildcard matching (* matches any characters)
fn wildcard_match(pattern: &str, text: &str) -> bool {
    // If no wildcards, must be exact match
    if !pattern.contains('*') {
        return pattern == text;
    }

    let parts: Vec<&str> = pattern.split('*').collect();
    let mut remaining_text = text;

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue; // Skip empty parts from consecutive wildcards
        }

        let is_first = i == 0;
        let is_last = i == parts.len() - 1;

        if is_first && !pattern.starts_with('*') {
            // Pattern starts with literal - must match at beginning
            if !remaining_text.starts_with(part) {
                return false;
            }
            remaining_text = &remaining_text[part.len()..];
        } else if is_last && !pattern.ends_with('*') {
            // Pattern ends with literal - must match at end
            if !remaining_text.ends_with(part) {
                return false;
            }
            // No need to update remaining_text, we're done
        } else {
            // Middle part - find it anywhere in remaining text
            if let Some(pos) = remaining_text.find(part) {
                remaining_text = &remaining_text[pos + part.len()..];
            } else {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_magic_values() {
        assert_eq!(Network::Mainnet.magic(), 0xE3E1F3E8);
        assert_eq!(Network::Testnet.magic(), 0xF4E5F3F4);
        assert_eq!(Network::Regtest.magic(), 0xDAB5BFFA);
    }

    #[test]
    fn test_network_default_ports() {
        assert_eq!(Network::Mainnet.default_port(), 8333);
        assert_eq!(Network::Testnet.default_port(), 18333);
        assert_eq!(Network::Regtest.default_port(), 18444);
    }

    #[test]
    fn test_network_from_str() {
        assert_eq!("mainnet".parse::<Network>().unwrap(), Network::Mainnet);
        assert_eq!("testnet".parse::<Network>().unwrap(), Network::Testnet);
        assert_eq!("regtest".parse::<Network>().unwrap(), Network::Regtest);

        // Case insensitive
        assert_eq!("MAINNET".parse::<Network>().unwrap(), Network::Mainnet);
        assert_eq!("TestNet".parse::<Network>().unwrap(), Network::Testnet);

        // Invalid network
        assert!("invalid".parse::<Network>().is_err());
    }

    #[test]
    fn test_network_display() {
        assert_eq!(Network::Mainnet.to_string(), "mainnet");
        assert_eq!(Network::Testnet.to_string(), "testnet");
        assert_eq!(Network::Regtest.to_string(), "regtest");
    }

    #[test]
    fn test_manager_config_defaults() {
        let config = ManagerConfig::default();

        assert_eq!(config.network, Network::Mainnet);
        assert_eq!(config.target_connections, 8);
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.default_port, 8333);
        assert!(!config.enable_listener);
        assert_eq!(config.listener_address, "0.0.0.0");
        assert!(!config.enable_telemetry);
    }

    #[test]
    fn test_manager_config_validation_rejects_invalid_limits() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.target_connections = 30;
        config.max_connections = 20;

        let result = config.validate();
        assert!(matches!(result, Err(Error::InvalidConnectionLimits { .. })));
    }

    #[test]
    fn test_manager_config_validation_accepts_equal_limits() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.target_connections = 20;
        config.max_connections = 20;

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_manager_config_validation_rejects_zero_target() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.target_connections = 0;

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_config_validation_rejects_zero_max() {
        let mut config = ManagerConfig::new(Network::Mainnet);
        config.max_connections = 0;

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_config_default_dns_seeds() {
        let mainnet_config = ManagerConfig::new(Network::Mainnet);
        assert!(!mainnet_config.dns_seeds.is_empty());

        let testnet_config = ManagerConfig::new(Network::Testnet);
        assert!(!testnet_config.dns_seeds.is_empty());

        let regtest_config = ManagerConfig::new(Network::Regtest);
        assert!(regtest_config.dns_seeds.is_empty());
    }

    #[test]
    fn test_connection_config_defaults() {
        let config = ConnectionConfig::default();

        assert_eq!(config.ping_interval, Duration::from_secs(5 * 60));
        assert_eq!(config.ping_timeout, Duration::from_secs(30));
        assert_eq!(config.handshake_timeout, Duration::from_secs(10));
        assert_eq!(config.initial_backoff, Duration::from_secs(5));
        assert_eq!(config.max_retries, 10);
        assert_eq!(config.backoff_multiplier, 2.0);
        assert_eq!(config.max_restarts, 3);
        assert_eq!(config.restart_window, Duration::from_secs(60 * 60));
    }

    #[test]
    fn test_connection_config_validation() {
        let config = ConnectionConfig::default();
        assert!(config.validate().is_ok());

        let mut bad_config = ConnectionConfig::default();
        bad_config.backoff_multiplier = 0.5;
        assert!(bad_config.validate().is_err());
    }

    #[test]
    fn test_user_agent_ban_simple_contains() {
        let banned = vec!["badclient".to_string(), "malicious".to_string()];

        assert!(is_user_agent_banned("/badclient:1.0/", &banned));
        assert!(is_user_agent_banned("/some-malicious-software:2.0/", &banned));
        assert!(!is_user_agent_banned("/goodclient:1.0/", &banned));
    }

    #[test]
    fn test_user_agent_ban_wildcard() {
        let banned = vec!["/banned*".to_string(), "*evil*".to_string()];

        assert!(is_user_agent_banned("/banned-client:1.0/", &banned));
        assert!(is_user_agent_banned("/some-evil-client:1.0/", &banned));
        assert!(!is_user_agent_banned("/good-client:1.0/", &banned));
    }

    #[test]
    fn test_wildcard_match() {
        // Exact match
        assert!(wildcard_match("test", "test"));
        assert!(!wildcard_match("test", "testing"));

        // Prefix wildcard
        assert!(wildcard_match("*test", "mytest"));
        assert!(wildcard_match("*test", "test"));
        assert!(!wildcard_match("*test", "testing"));

        // Suffix wildcard
        assert!(wildcard_match("test*", "testing"));
        assert!(wildcard_match("test*", "test"));
        assert!(!wildcard_match("test*", "mytest"));

        // Both wildcards
        assert!(wildcard_match("*test*", "mytesting"));
        assert!(wildcard_match("*test*", "test"));

        // Middle wildcard
        assert!(wildcard_match("a*b", "ab"));
        assert!(wildcard_match("a*b", "axxxb"));
        assert!(!wildcard_match("a*b", "axxx"));
    }

    #[test]
    fn test_network_serialization() {
        let network = Network::Mainnet;
        let json = serde_json::to_string(&network).unwrap();
        let deserialized: Network = serde_json::from_str(&json).unwrap();
        assert_eq!(network, deserialized);
    }
}
