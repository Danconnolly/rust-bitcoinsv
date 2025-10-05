// Integration tests for P2P module
// These tests require a real BSV node to connect to
// Set BSV_TEST_NODE environment variable to override default node
// Example: BSV_TEST_NODE=192.168.1.100:8333 cargo test --test p2p_integration_tests

mod common;

use bitcoinsv::p2p::{Message, MessageHeader, MAGIC_MAINNET};
use bytes::BytesMut;
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
