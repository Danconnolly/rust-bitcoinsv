use bitcoinsv::p2p::{NetworkAddress, Services, VersionMessage, PROTOCOL_VERSION};
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

/// Create a test version message for handshake
pub fn create_test_version_message(peer_addr: IpAddr, peer_port: u16) -> VersionMessage {
    VersionMessage {
        version: PROTOCOL_VERSION,
        services: Services::NETWORK,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        recv_addr: NetworkAddress {
            timestamp: None,
            services: Services::NONE,
            addr: peer_addr,
            port: peer_port,
        },
        from_addr: NetworkAddress {
            timestamp: None,
            services: Services::NETWORK,
            addr: IpAddr::V4([127, 0, 0, 1].into()),
            port: 8333,
        },
        nonce: rand::random(),
        user_agent: "/rust-bitcoinsv-test:0.1.0/".to_string(),
        start_height: 0,
        relay: true,
    }
}

/// Initialize test logging (call once per test suite)
pub fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into()),
        )
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_version_message() {
        let peer_addr = IpAddr::V4([192, 168, 1, 1].into());
        let version_msg = create_test_version_message(peer_addr, 8333);

        assert_eq!(version_msg.version, PROTOCOL_VERSION);
        assert_eq!(version_msg.services, Services::NETWORK);
        assert_eq!(version_msg.recv_addr.addr, peer_addr);
        assert_eq!(version_msg.recv_addr.port, 8333);
        assert!(version_msg.user_agent.contains("rust-bitcoinsv"));
    }
}
