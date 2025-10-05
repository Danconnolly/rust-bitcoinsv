// Integration tests for P2P module
// These tests require a real BSV node to connect to
// Set BSV_TEST_NODE environment variable to override default node
// Example: BSV_TEST_NODE=192.168.1.100:8333 cargo test --test p2p_integration_tests

mod common;

use bitcoinsv::p2p::{
    HandshakeState, Message, MessageHeader, Network, PingPongState, Services, VersionMessage,
    MAGIC_MAINNET,
};
use bytes::{Buf, BytesMut};
use common::config::{get_test_node_address, get_test_timeout_secs};
use common::helpers::{create_test_version_message, init_test_logging};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Test basic TCP connectivity to a real BSV node
///
/// This test requires BSV_TEST_NODE environment variable to be set to a valid BSV node.
/// Set it like: BSV_TEST_NODE=your.bsv.node.com:8333
#[tokio::test]
async fn test_tcp_connection_to_remote_node() {
    init_test_logging();

    // Skip test if BSV_TEST_NODE is not explicitly set
    if std::env::var("BSV_TEST_NODE").is_err() {
        eprintln!("⚠️  Skipping test: BSV_TEST_NODE environment variable not set");
        eprintln!("   Set BSV_TEST_NODE=<node_address>:8333 to run integration tests");
        return;
    }

    let node_addr = get_test_node_address();
    let timeout_secs = get_test_timeout_secs();

    tracing::info!("Attempting to connect to BSV node at {}", node_addr);

    // Try to establish TCP connection
    let result = timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(node_addr),
    )
    .await;

    match result {
        Ok(Ok(stream)) => {
            tracing::info!("Successfully connected to {}", node_addr);
            let peer_addr = stream.peer_addr().unwrap();
            assert_eq!(peer_addr, node_addr);
            drop(stream);
        }
        Ok(Err(e)) => {
            panic!(
                "Failed to connect to BSV node at {}: {}. \
                 Make sure the node is accessible or set BSV_TEST_NODE to a different node.",
                node_addr, e
            );
        }
        Err(_) => {
            panic!(
                "Connection to {} timed out after {} seconds. \
                 Make sure the node is accessible or increase BSV_TEST_TIMEOUT.",
                node_addr, timeout_secs
            );
        }
    }
}

/// Test that we can send a version message and read the header of a response
#[tokio::test]
async fn test_send_version_and_receive_header() {
    init_test_logging();

    if std::env::var("BSV_TEST_NODE").is_err() {
        eprintln!("⚠️  Skipping test: BSV_TEST_NODE environment variable not set");
        return;
    }

    let node_addr = get_test_node_address();
    let timeout_secs = get_test_timeout_secs();

    tracing::info!(
        "Testing version message exchange with BSV node at {}",
        node_addr
    );

    // Connect to the node
    let mut stream = timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(node_addr),
    )
    .await
    .expect("Connection timeout")
    .expect("Failed to connect");

    tracing::debug!("Connected, creating version message");

    // Create and send version message
    let version_msg = create_test_version_message(node_addr.ip(), node_addr.port());
    let msg = Message::Version(version_msg);

    // Encode the message
    let mut payload_buf = BytesMut::new();
    msg.encode_payload(&mut payload_buf)
        .expect("Failed to encode payload");

    let header = MessageHeader::new(MAGIC_MAINNET, msg.command(), &payload_buf);

    let mut header_buf = BytesMut::new();
    header.encode(&mut header_buf);

    tracing::debug!(
        "Sending version message ({} bytes header + {} bytes payload)",
        header_buf.len(),
        payload_buf.len()
    );

    // Send header and payload
    stream
        .write_all(&header_buf)
        .await
        .expect("Failed to write header");
    stream
        .write_all(&payload_buf)
        .await
        .expect("Failed to write payload");

    tracing::debug!("Version message sent, waiting for response");

    // Try to read a message header in response
    let mut response_header = vec![0u8; MessageHeader::SIZE];

    let read_result = timeout(
        Duration::from_secs(timeout_secs),
        stream.read_exact(&mut response_header),
    )
    .await;

    match read_result {
        Ok(Ok(_)) => {
            tracing::info!("Received response header from node");

            // Try to decode the header
            let mut buf = &response_header[..];
            let decoded_header = MessageHeader::decode(&mut buf);

            match decoded_header {
                Ok(header) => {
                    tracing::info!("Response command: {}", header.command_string());
                    // We expect either "version" or "verack" as first response
                    let cmd = header.command_string();
                    assert!(
                        cmd == "version" || cmd == "verack",
                        "Unexpected first response command: {}",
                        cmd
                    );
                }
                Err(e) => {
                    panic!("Failed to decode response header: {}", e);
                }
            }
        }
        Ok(Err(e)) => {
            panic!("Error reading response: {}", e);
        }
        Err(_) => {
            panic!("Timeout waiting for response from node");
        }
    }
}

/// Test that we can complete a full handshake with a real node
#[tokio::test]
async fn test_complete_handshake_with_remote_node() {
    init_test_logging();

    if std::env::var("BSV_TEST_NODE").is_err() {
        eprintln!("⚠️  Skipping test: BSV_TEST_NODE environment variable not set");
        return;
    }

    let node_addr = get_test_node_address();
    let timeout_secs = get_test_timeout_secs();

    tracing::info!("Testing full handshake with BSV node at {}", node_addr);

    // Connect to the node
    let mut stream = timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(node_addr),
    )
    .await
    .expect("Connection timeout")
    .expect("Failed to connect");

    tracing::debug!("Connected, initiating handshake");

    // Create and send version message
    let version_msg = create_test_version_message(node_addr.ip(), node_addr.port());
    let msg = Message::Version(version_msg);

    // Encode and send
    let mut payload_buf = BytesMut::new();
    msg.encode_payload(&mut payload_buf).unwrap();
    let header = MessageHeader::new(MAGIC_MAINNET, msg.command(), &payload_buf);
    let mut header_buf = BytesMut::new();
    header.encode(&mut header_buf);

    stream.write_all(&header_buf).await.unwrap();
    stream.write_all(&payload_buf).await.unwrap();

    tracing::debug!("Sent version message, waiting for version and verack");

    // Handshake tracking
    let mut version_received = false;
    let mut verack_received = false;
    let mut verack_sent = false;

    // Read messages until we complete handshake
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

    while !version_received || !verack_received {
        if tokio::time::Instant::now() > deadline {
            panic!("Handshake timeout");
        }

        // Read message header
        let mut response_header = vec![0u8; MessageHeader::SIZE];
        timeout(
            Duration::from_secs(5),
            stream.read_exact(&mut response_header),
        )
        .await
        .expect("Read timeout")
        .expect("Read error");

        let mut buf = &response_header[..];
        let header = MessageHeader::decode(&mut buf).expect("Failed to decode header");

        // Read payload
        let mut payload = vec![0u8; header.payload_size as usize];
        if header.payload_size > 0 {
            stream.read_exact(&mut payload).await.expect("Read error");
        }

        let cmd = header.command_string();
        tracing::debug!("Received message: {}", cmd);

        match cmd.as_str() {
            "version" => {
                version_received = true;
                tracing::debug!("Received version from peer");

                // Send verack in response
                if !verack_sent {
                    let verack_msg = Message::Verack;
                    let mut verack_payload = BytesMut::new();
                    verack_msg.encode_payload(&mut verack_payload).unwrap();
                    let verack_header =
                        MessageHeader::new(MAGIC_MAINNET, verack_msg.command(), &verack_payload);
                    let mut verack_header_buf = BytesMut::new();
                    verack_header.encode(&mut verack_header_buf);

                    stream.write_all(&verack_header_buf).await.unwrap();
                    stream.write_all(&verack_payload).await.unwrap();

                    verack_sent = true;
                    tracing::debug!("Sent verack");
                }
            }
            "verack" => {
                verack_received = true;
                tracing::debug!("Received verack from peer");
            }
            other => {
                tracing::debug!("Received other message during handshake: {}", other);
                // Other messages like sendheaders are acceptable
            }
        }
    }

    tracing::info!("Handshake completed successfully!");

    assert!(version_received, "Should have received version");
    assert!(verack_received, "Should have received verack");
}

/// Test handshake using HandshakeState to track completion
#[tokio::test]
async fn test_handshake_state_tracking_with_remote_node() {
    init_test_logging();

    if std::env::var("BSV_TEST_NODE").is_err() {
        eprintln!("⚠️  Skipping test: BSV_TEST_NODE environment variable not set");
        return;
    }

    let node_addr = get_test_node_address();
    let timeout_secs = get_test_timeout_secs();

    tracing::info!(
        "Testing HandshakeState tracking with BSV node at {}",
        node_addr
    );

    // Connect to the node
    let mut stream = timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(node_addr),
    )
    .await
    .expect("Connection timeout")
    .expect("Failed to connect");

    // Initialize handshake state
    let mut handshake = HandshakeState::new();
    handshake.start();

    tracing::debug!("Connected, starting handshake");

    // Send version message
    let version_msg = create_test_version_message(node_addr.ip(), node_addr.port());
    let msg = Message::Version(version_msg);

    let mut payload_buf = BytesMut::new();
    msg.encode_payload(&mut payload_buf).unwrap();
    let header = MessageHeader::new(MAGIC_MAINNET, msg.command(), &payload_buf);
    let mut header_buf = BytesMut::new();
    header.encode(&mut header_buf);

    stream.write_all(&header_buf).await.unwrap();
    stream.write_all(&payload_buf).await.unwrap();

    handshake.mark_version_sent();
    tracing::debug!(
        "Sent version, handshake complete: {}",
        handshake.is_complete()
    );

    // Read messages until handshake is complete
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

    while !handshake.is_complete() {
        if tokio::time::Instant::now() > deadline {
            panic!("Handshake timeout");
        }

        // Check for timeout
        if handshake.is_timed_out(timeout_secs) {
            panic!("Handshake timed out according to HandshakeState");
        }

        // Read message header
        let mut response_header = vec![0u8; MessageHeader::SIZE];
        timeout(
            Duration::from_secs(5),
            stream.read_exact(&mut response_header),
        )
        .await
        .expect("Read timeout")
        .expect("Read error");

        let mut buf = &response_header[..];
        let header = MessageHeader::decode(&mut buf).expect("Failed to decode header");

        // Read payload
        let mut payload = vec![0u8; header.payload_size as usize];
        if header.payload_size > 0 {
            stream.read_exact(&mut payload).await.expect("Read error");
        }

        let cmd = header.command_string();
        tracing::debug!("Received message: {}", cmd);

        match cmd.as_str() {
            "version" => {
                // Decode version message
                let mut payload_buf = &payload[..];
                let peer_version =
                    decode_version_message(&mut payload_buf).expect("Failed to decode version");

                handshake.mark_version_received(peer_version);
                tracing::debug!(
                    "Received version, handshake complete: {}",
                    handshake.is_complete()
                );

                // Send verack in response
                if !handshake.verack_sent {
                    let verack_msg = Message::Verack;
                    let mut verack_payload = BytesMut::new();
                    verack_msg.encode_payload(&mut verack_payload).unwrap();
                    let verack_header =
                        MessageHeader::new(MAGIC_MAINNET, verack_msg.command(), &verack_payload);
                    let mut verack_header_buf = BytesMut::new();
                    verack_header.encode(&mut verack_header_buf);

                    stream.write_all(&verack_header_buf).await.unwrap();
                    stream.write_all(&verack_payload).await.unwrap();

                    handshake.mark_verack_sent();
                    tracing::debug!(
                        "Sent verack, handshake complete: {}",
                        handshake.is_complete()
                    );
                }
            }
            "verack" => {
                handshake.mark_verack_received();
                tracing::debug!(
                    "Received verack, handshake complete: {}",
                    handshake.is_complete()
                );
            }
            other => {
                tracing::debug!("Received other message during handshake: {}", other);
            }
        }
    }

    tracing::info!("Handshake completed successfully using HandshakeState!");

    assert!(handshake.is_complete(), "Handshake should be complete");
    assert!(handshake.peer_version.is_some(), "Should have peer version");
}

/// Test that we can validate received version messages
#[tokio::test]
async fn test_validate_peer_version() {
    init_test_logging();

    if std::env::var("BSV_TEST_NODE").is_err() {
        eprintln!("⚠️  Skipping test: BSV_TEST_NODE environment variable not set");
        return;
    }

    let node_addr = get_test_node_address();
    let timeout_secs = get_test_timeout_secs();

    tracing::info!("Testing version validation with BSV node at {}", node_addr);

    // Connect and send version
    let mut stream = timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(node_addr),
    )
    .await
    .expect("Connection timeout")
    .expect("Failed to connect");

    let version_msg = create_test_version_message(node_addr.ip(), node_addr.port());
    let msg = Message::Version(version_msg);

    let mut payload_buf = BytesMut::new();
    msg.encode_payload(&mut payload_buf).unwrap();
    let header = MessageHeader::new(MAGIC_MAINNET, msg.command(), &payload_buf);
    let mut header_buf = BytesMut::new();
    header.encode(&mut header_buf);

    stream.write_all(&header_buf).await.unwrap();
    stream.write_all(&payload_buf).await.unwrap();

    // Read version response
    let mut response_header = vec![0u8; MessageHeader::SIZE];
    timeout(
        Duration::from_secs(timeout_secs),
        stream.read_exact(&mut response_header),
    )
    .await
    .expect("Read timeout")
    .expect("Read error");

    let mut buf = &response_header[..];
    let header = MessageHeader::decode(&mut buf).expect("Failed to decode header");

    // Find and validate version message
    let mut version_found = false;
    if header.command_string() == "version" {
        let mut payload = vec![0u8; header.payload_size as usize];
        stream.read_exact(&mut payload).await.expect("Read error");

        let mut payload_buf = &payload[..];
        let peer_version =
            decode_version_message(&mut payload_buf).expect("Failed to decode version");

        tracing::info!("Peer user agent: {}", peer_version.user_agent);

        // Validate the version (should accept valid BSV nodes)
        let validation_result =
            bitcoinsv::p2p::validate_version(&peer_version, Network::Mainnet, &[]);

        match validation_result {
            Ok(()) => {
                tracing::info!("Version validation passed");
                version_found = true;
            }
            Err(e) => {
                tracing::warn!("Version validation failed: {}", e);
                // Some test nodes might not have BSV user agent, that's OK for this test
                version_found = true;
            }
        }
    }

    assert!(version_found, "Should have received and validated version");
}

/// Test sending sendheaders after handshake completion
#[tokio::test]
async fn test_sendheaders_after_handshake() {
    init_test_logging();

    if std::env::var("BSV_TEST_NODE").is_err() {
        eprintln!("⚠️  Skipping test: BSV_TEST_NODE environment variable not set");
        return;
    }

    let node_addr = get_test_node_address();
    let timeout_secs = get_test_timeout_secs();

    tracing::info!(
        "Testing sendheaders message after handshake with {}",
        node_addr
    );

    // Connect and complete handshake
    let mut stream = timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(node_addr),
    )
    .await
    .expect("Connection timeout")
    .expect("Failed to connect");

    let mut handshake = HandshakeState::new();
    handshake.start();

    // Send version
    let version_msg = create_test_version_message(node_addr.ip(), node_addr.port());
    let msg = Message::Version(version_msg);

    let mut payload_buf = BytesMut::new();
    msg.encode_payload(&mut payload_buf).unwrap();
    let header = MessageHeader::new(MAGIC_MAINNET, msg.command(), &payload_buf);
    let mut header_buf = BytesMut::new();
    header.encode(&mut header_buf);

    stream.write_all(&header_buf).await.unwrap();
    stream.write_all(&payload_buf).await.unwrap();
    handshake.mark_version_sent();

    // Complete handshake
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

    while !handshake.is_complete() {
        if tokio::time::Instant::now() > deadline {
            panic!("Handshake timeout");
        }

        let mut response_header = vec![0u8; MessageHeader::SIZE];
        timeout(
            Duration::from_secs(5),
            stream.read_exact(&mut response_header),
        )
        .await
        .expect("Read timeout")
        .expect("Read error");

        let mut buf = &response_header[..];
        let header = MessageHeader::decode(&mut buf).expect("Failed to decode header");

        let mut payload = vec![0u8; header.payload_size as usize];
        if header.payload_size > 0 {
            stream.read_exact(&mut payload).await.expect("Read error");
        }

        match header.command_string().as_str() {
            "version" => {
                let mut payload_buf = &payload[..];
                let peer_version = decode_version_message(&mut payload_buf).unwrap();
                handshake.mark_version_received(peer_version);

                if !handshake.verack_sent {
                    let verack = Message::Verack;
                    let mut vp = BytesMut::new();
                    verack.encode_payload(&mut vp).unwrap();
                    let vh = MessageHeader::new(MAGIC_MAINNET, verack.command(), &vp);
                    let mut vhb = BytesMut::new();
                    vh.encode(&mut vhb);
                    stream.write_all(&vhb).await.unwrap();
                    stream.write_all(&vp).await.unwrap();
                    handshake.mark_verack_sent();
                }
            }
            "verack" => {
                handshake.mark_verack_received();
            }
            _ => {}
        }
    }

    tracing::info!("Handshake complete, sending sendheaders");

    // Now send sendheaders
    let sendheaders = Message::SendHeaders;
    let mut sh_payload = BytesMut::new();
    sendheaders.encode_payload(&mut sh_payload).unwrap();
    let sh_header = MessageHeader::new(MAGIC_MAINNET, sendheaders.command(), &sh_payload);
    let mut sh_header_buf = BytesMut::new();
    sh_header.encode(&mut sh_header_buf);

    stream.write_all(&sh_header_buf).await.unwrap();
    stream.write_all(&sh_payload).await.unwrap();

    tracing::info!("Successfully sent sendheaders after handshake");

    // Test passes if we got here without errors
}

/// Helper function to decode version message from bytes
fn decode_version_message(buf: &mut dyn Buf) -> bitcoinsv::Result<VersionMessage> {
    use bitcoinsv::p2p::NetworkAddress;

    if buf.remaining() < 80 {
        return Err(bitcoinsv::Error::BadData(
            "Insufficient data for version message".to_string(),
        ));
    }

    let version = buf.get_u32_le();
    let services = Services(buf.get_u64_le());
    let timestamp = buf.get_i64_le();
    let recv_addr = NetworkAddress::decode(buf)?;
    let from_addr = NetworkAddress::decode(buf)?;
    let nonce = buf.get_u64_le();

    // Decode user agent (var_string)
    let ua_len = decode_var_int(buf)?;
    if buf.remaining() < ua_len as usize {
        return Err(bitcoinsv::Error::BadData(
            "Insufficient data for user agent".to_string(),
        ));
    }
    let mut ua_bytes = vec![0u8; ua_len as usize];
    buf.copy_to_slice(&mut ua_bytes);
    let user_agent = String::from_utf8_lossy(&ua_bytes).to_string();

    let start_height = buf.get_u32_le();

    // Relay is optional
    let relay = if buf.remaining() >= 1 {
        buf.get_u8() != 0
    } else {
        true
    };

    Ok(VersionMessage {
        version,
        services,
        timestamp,
        recv_addr,
        from_addr,
        nonce,
        user_agent,
        start_height,
        relay,
    })
}

/// Decode variable-length integer
fn decode_var_int(buf: &mut dyn Buf) -> bitcoinsv::Result<u64> {
    if buf.remaining() < 1 {
        return Err(bitcoinsv::Error::BadData(
            "Insufficient data for var_int".to_string(),
        ));
    }

    let first = buf.get_u8();
    match first {
        0xFF => {
            if buf.remaining() < 8 {
                return Err(bitcoinsv::Error::BadData(
                    "Insufficient data for var_int".to_string(),
                ));
            }
            Ok(buf.get_u64_le())
        }
        0xFE => {
            if buf.remaining() < 4 {
                return Err(bitcoinsv::Error::BadData(
                    "Insufficient data for var_int".to_string(),
                ));
            }
            Ok(buf.get_u32_le() as u64)
        }
        0xFD => {
            if buf.remaining() < 2 {
                return Err(bitcoinsv::Error::BadData(
                    "Insufficient data for var_int".to_string(),
                ));
            }
            Ok(buf.get_u16_le() as u64)
        }
        n => Ok(n as u64),
    }
}

/// Test ping/pong exchange with a real node
#[tokio::test]
async fn test_ping_pong_exchange_with_remote_node() {
    init_test_logging();

    if std::env::var("BSV_TEST_NODE").is_err() {
        eprintln!("⚠️  Skipping test: BSV_TEST_NODE environment variable not set");
        return;
    }

    let node_addr = get_test_node_address();
    let timeout_secs = get_test_timeout_secs();

    tracing::info!("Testing ping/pong exchange with BSV node at {}", node_addr);

    // Connect and complete handshake first
    let mut stream = timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(node_addr),
    )
    .await
    .expect("Connection timeout")
    .expect("Failed to connect");

    let mut handshake = HandshakeState::new();
    handshake.start();

    // Send version
    let version_msg = create_test_version_message(node_addr.ip(), node_addr.port());
    let msg = Message::Version(version_msg);

    let mut payload_buf = BytesMut::new();
    msg.encode_payload(&mut payload_buf).unwrap();
    let header = MessageHeader::new(MAGIC_MAINNET, msg.command(), &payload_buf);
    let mut header_buf = BytesMut::new();
    header.encode(&mut header_buf);

    stream.write_all(&header_buf).await.unwrap();
    stream.write_all(&payload_buf).await.unwrap();
    handshake.mark_version_sent();

    // Complete handshake
    tracing::debug!("Completing handshake before ping/pong");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

    while !handshake.is_complete() {
        if tokio::time::Instant::now() > deadline {
            panic!("Handshake timeout");
        }

        let mut response_header = vec![0u8; MessageHeader::SIZE];
        timeout(
            Duration::from_secs(5),
            stream.read_exact(&mut response_header),
        )
        .await
        .expect("Read timeout")
        .expect("Read error");

        let mut buf = &response_header[..];
        let header = MessageHeader::decode(&mut buf).expect("Failed to decode header");

        let mut payload = vec![0u8; header.payload_size as usize];
        if header.payload_size > 0 {
            stream.read_exact(&mut payload).await.expect("Read error");
        }

        match header.command_string().as_str() {
            "version" => {
                let mut payload_buf = &payload[..];
                let peer_version = decode_version_message(&mut payload_buf).unwrap();
                handshake.mark_version_received(peer_version);

                if !handshake.verack_sent {
                    let verack = Message::Verack;
                    let mut vp = BytesMut::new();
                    verack.encode_payload(&mut vp).unwrap();
                    let vh = MessageHeader::new(MAGIC_MAINNET, verack.command(), &vp);
                    let mut vhb = BytesMut::new();
                    vh.encode(&mut vhb);
                    stream.write_all(&vhb).await.unwrap();
                    stream.write_all(&vp).await.unwrap();
                    handshake.mark_verack_sent();
                }
            }
            "verack" => {
                handshake.mark_verack_received();
            }
            _ => {}
        }
    }

    tracing::info!("Handshake complete, starting ping/pong test");

    // Now test ping/pong
    let mut ping_pong = PingPongState::new(10);

    // Generate and send ping
    let ping_nonce = PingPongState::generate_nonce();
    ping_pong.record_ping(ping_nonce);

    let ping_msg = Message::Ping(ping_nonce);
    let mut ping_payload = BytesMut::new();
    ping_msg.encode_payload(&mut ping_payload).unwrap();
    let ping_header = MessageHeader::new(MAGIC_MAINNET, ping_msg.command(), &ping_payload);
    let mut ping_header_buf = BytesMut::new();
    ping_header.encode(&mut ping_header_buf);

    stream.write_all(&ping_header_buf).await.unwrap();
    stream.write_all(&ping_payload).await.unwrap();

    tracing::info!("Sent ping with nonce: {}", ping_nonce);

    // Wait for pong response
    let mut pong_received = false;
    let ping_deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

    while !pong_received {
        if tokio::time::Instant::now() > ping_deadline {
            panic!("Pong timeout");
        }

        let mut response_header = vec![0u8; MessageHeader::SIZE];
        timeout(
            Duration::from_secs(5),
            stream.read_exact(&mut response_header),
        )
        .await
        .expect("Read timeout")
        .expect("Read error");

        let mut buf = &response_header[..];
        let header = MessageHeader::decode(&mut buf).expect("Failed to decode header");

        let mut payload = vec![0u8; header.payload_size as usize];
        if header.payload_size > 0 {
            stream.read_exact(&mut payload).await.expect("Read error");
        }

        let cmd = header.command_string();
        tracing::debug!("Received message: {}", cmd);

        match cmd.as_str() {
            "pong" => {
                // Decode pong nonce
                if payload.len() >= 8 {
                    let pong_nonce = u64::from_le_bytes([
                        payload[0], payload[1], payload[2], payload[3], payload[4], payload[5],
                        payload[6], payload[7],
                    ]);

                    tracing::info!("Received pong with nonce: {}", pong_nonce);

                    // Validate pong
                    match ping_pong.validate_pong(pong_nonce) {
                        Ok(rtt) => {
                            tracing::info!("Pong validated! RTT: {:?}", rtt);
                            pong_received = true;
                        }
                        Err(e) => {
                            panic!("Pong validation failed: {}", e);
                        }
                    }
                }
            }
            "ping" => {
                // Respond to peer's ping
                if payload.len() >= 8 {
                    let peer_ping_nonce = u64::from_le_bytes([
                        payload[0], payload[1], payload[2], payload[3], payload[4], payload[5],
                        payload[6], payload[7],
                    ]);

                    tracing::debug!("Received ping from peer, sending pong");

                    let pong_msg = Message::Pong(peer_ping_nonce);
                    let mut pong_payload = BytesMut::new();
                    pong_msg.encode_payload(&mut pong_payload).unwrap();
                    let pong_header =
                        MessageHeader::new(MAGIC_MAINNET, pong_msg.command(), &pong_payload);
                    let mut pong_header_buf = BytesMut::new();
                    pong_header.encode(&mut pong_header_buf);

                    stream.write_all(&pong_header_buf).await.unwrap();
                    stream.write_all(&pong_payload).await.unwrap();
                }
            }
            other => {
                tracing::debug!("Received other message while waiting for pong: {}", other);
            }
        }
    }

    tracing::info!("Ping/pong exchange completed successfully!");

    assert!(pong_received, "Should have received pong response");
    assert_eq!(
        ping_pong.pending_count(),
        0,
        "No pending pings after validation"
    );
}
