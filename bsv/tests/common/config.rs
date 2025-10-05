use std::net::{SocketAddr, ToSocketAddrs};

/// Get the BSV test node address from environment or use default
/// Supports both IP:PORT and hostname:PORT formats
pub fn get_test_node_address() -> SocketAddr {
    let addr_str = std::env::var("BSV_TEST_NODE").unwrap_or_else(|_| {
        // Default to Bitcoin ABC seed node (also supports BSV protocol)
        // Users should set BSV_TEST_NODE to a known BSV node for real testing
        "seed.bitcoinabc.org:8333".to_string()
    });

    // Try to parse directly as SocketAddr first
    if let Ok(addr) = addr_str.parse::<SocketAddr>() {
        return addr;
    }

    // If that fails, try DNS resolution
    addr_str
        .to_socket_addrs()
        .expect("Invalid BSV_TEST_NODE address format. Expected format: IP:PORT or hostname:PORT")
        .next()
        .expect("DNS resolution returned no addresses")
}

/// Get the network type for testing from environment or use default
pub fn get_test_network() -> String {
    std::env::var("BSV_TEST_NETWORK").unwrap_or_else(|_| "mainnet".to_string())
}

/// Get test timeout duration in seconds
pub fn get_test_timeout_secs() -> u64 {
    std::env::var("BSV_TEST_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30) // 30 second default timeout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        // Should not panic with defaults
        let _addr = get_test_node_address();
        let network = get_test_network();
        assert_eq!(network, "mainnet");

        let timeout = get_test_timeout_secs();
        assert_eq!(timeout, 30);
    }
}
