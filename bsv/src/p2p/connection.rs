//! Peer connection state machine and actor
//!
//! Manages the lifecycle of a single peer connection, including:
//! - Connection establishment
//! - Handshake
//! - Message exchange
//! - Reconnection and backoff
//! - Failure handling

use crate::p2p::{
    ConnectionConfig, HandshakeState, ManagerConfig, Message, MessageFramer, MessageHeader,
    NetworkAddress, Peer, PingPongState, Services, VersionMessage, PROTOCOL_VERSION,
};
use crate::{Error, Result};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{interval, timeout};
use tracing;

// Re-export types from manager module to avoid duplication
pub use crate::p2p::manager::{
    BitcoinMessageEvent, ConnectionSlots, ControlEvent, PeerConnectionCommand, PeerConnectionHandle,
};

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
    pub fn new_outbound(
        peer: Peer,
        restart_config: (usize, Duration),
        ping_timeout_secs: u64,
    ) -> Self {
        Self {
            peer,
            state: ConnectionState::Disconnected,
            connection_type: ConnectionType::Outbound,
            handshake: HandshakeState::new(),
            ping_pong: PingPongState::new(ping_timeout_secs),
            restart_tracking: RestartTracking::new(restart_config.0, restart_config.1.as_secs()),
            reject_after_handshake: false,
            send_headers_mode: false,
        }
    }

    /// Create a new inbound connection
    pub fn new_inbound(peer: Peer, ping_timeout_secs: u64) -> Self {
        Self {
            peer,
            state: ConnectionState::Disconnected,
            connection_type: ConnectionType::Inbound,
            handshake: HandshakeState::new(),
            ping_pong: PingPongState::new(ping_timeout_secs),
            restart_tracking: RestartTracking::new(0, 0), // Inbound doesn't restart
            reject_after_handshake: false,
            send_headers_mode: false,
        }
    }

    /// Create an over-capacity connection (will reject after handshake)
    pub fn new_over_capacity(peer: Peer, ping_timeout_secs: u64) -> Self {
        Self {
            peer,
            state: ConnectionState::Disconnected,
            connection_type: ConnectionType::OverCapacity,
            handshake: HandshakeState::new(),
            ping_pong: PingPongState::new(ping_timeout_secs),
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

/// Peer connection actor
///
/// This async actor manages the lifecycle of a single peer connection.
pub struct PeerConnectionActor {
    connection: PeerConnection,
    config: ConnectionConfig,
    network: crate::p2p::Network,
    tcp_stream: Option<TcpStream>,
    framer: MessageFramer,
    control_rx: mpsc::Receiver<PeerConnectionCommand>,
    control_event_tx: broadcast::Sender<ControlEvent>,
    bitcoin_message_tx: broadcast::Sender<BitcoinMessageEvent>,
    connection_slots: Arc<ConnectionSlots>,
}

impl PeerConnectionActor {
    /// Spawn an outbound connection actor
    pub fn spawn_outbound(
        peer: Peer,
        config: ConnectionConfig,
        manager_config: &ManagerConfig,
        control_event_tx: broadcast::Sender<ControlEvent>,
        bitcoin_message_tx: broadcast::Sender<BitcoinMessageEvent>,
        connection_slots: Arc<ConnectionSlots>,
    ) -> PeerConnectionHandle {
        let (control_tx, control_rx) = mpsc::channel(32);
        let peer_id = peer.id;

        // Try to reserve a connection slot
        if !connection_slots.try_reserve() {
            // No slots available, immediately fail
            let _ = control_event_tx.send(ControlEvent::ConnectionFailed {
                peer_id,
                reason: "No connection slots available".to_string(),
            });
            return PeerConnectionHandle {
                peer_id,
                control_tx,
            };
        }

        let actor = Self {
            connection: PeerConnection::new_outbound(
                peer,
                (config.max_restarts, config.restart_window),
                config.ping_timeout.as_secs(),
            ),
            config,
            network: manager_config.network,
            tcp_stream: None,
            framer: MessageFramer::new(),
            control_rx,
            control_event_tx,
            bitcoin_message_tx,
            connection_slots,
        };

        tokio::spawn(async move {
            actor.run().await;
        });

        PeerConnectionHandle {
            peer_id,
            control_tx,
        }
    }

    /// Spawn an inbound connection actor
    pub fn spawn_inbound(
        peer: Peer,
        stream: TcpStream,
        config: ConnectionConfig,
        manager_config: &ManagerConfig,
        control_event_tx: broadcast::Sender<ControlEvent>,
        bitcoin_message_tx: broadcast::Sender<BitcoinMessageEvent>,
        connection_slots: Arc<ConnectionSlots>,
    ) -> PeerConnectionHandle {
        let (control_tx, control_rx) = mpsc::channel(32);
        let peer_id = peer.id;

        let actor = Self {
            connection: PeerConnection::new_inbound(peer, config.ping_timeout.as_secs()),
            config,
            network: manager_config.network,
            tcp_stream: Some(stream),
            framer: MessageFramer::new(),
            control_rx,
            control_event_tx,
            bitcoin_message_tx,
            connection_slots,
        };

        tokio::spawn(async move {
            actor.run().await;
        });

        PeerConnectionHandle {
            peer_id,
            control_tx,
        }
    }

    /// Spawn an over-capacity connection actor (will reject after handshake)
    pub fn spawn_over_capacity(
        peer: Peer,
        stream: TcpStream,
        config: ConnectionConfig,
        manager_config: &ManagerConfig,
        control_event_tx: broadcast::Sender<ControlEvent>,
        bitcoin_message_tx: broadcast::Sender<BitcoinMessageEvent>,
        connection_slots: Arc<ConnectionSlots>,
    ) -> PeerConnectionHandle {
        let (control_tx, control_rx) = mpsc::channel(32);
        let peer_id = peer.id;

        let actor = Self {
            connection: PeerConnection::new_over_capacity(peer, config.ping_timeout.as_secs()),
            config,
            network: manager_config.network,
            tcp_stream: Some(stream),
            framer: MessageFramer::new(),
            control_rx,
            control_event_tx,
            bitcoin_message_tx,
            connection_slots,
        };

        tokio::spawn(async move {
            actor.run().await;
        });

        PeerConnectionHandle {
            peer_id,
            control_tx,
        }
    }

    /// Main actor loop
    async fn run(mut self) {
        let peer_id = self.connection.peer.id;

        tracing::info!(
            peer_id = %peer_id,
            peer_addr = %self.connection.peer.socket_addr(),
            connection_type = ?self.connection.connection_type,
            "Starting peer connection actor"
        );

        let result = self.run_connection().await;

        if let Err(e) = result {
            tracing::error!(
                peer_id = %peer_id,
                error = %e,
                "Connection failed"
            );

            let _ = self.control_event_tx.send(ControlEvent::ConnectionFailed {
                peer_id,
                reason: e.to_string(),
            });
        }

        // Release connection slot
        self.connection_slots.release();

        tracing::info!(
            peer_id = %peer_id,
            "Peer connection actor terminated"
        );
    }

    /// Run the connection lifecycle
    async fn run_connection(&mut self) -> Result<()> {
        // If we don't have a TCP stream yet (outbound), establish it
        if self.tcp_stream.is_none() {
            self.establish_outbound_connection().await?;
        }

        // Perform handshake
        self.perform_handshake().await?;

        // If reject_after_handshake is set, reject now
        if self.connection.reject_after_handshake {
            self.connection.transition_to_rejected()?;
            return Ok(());
        }

        // Transition to connected
        self.connection.transition_to_connected()?;

        let _ = self
            .control_event_tx
            .send(ControlEvent::ConnectionEstablished {
                peer_id: self.connection.peer.id,
            });

        // Enter message loop
        self.message_loop().await?;

        Ok(())
    }

    /// Establish outbound TCP connection
    async fn establish_outbound_connection(&mut self) -> Result<()> {
        let addr = self.connection.peer.socket_addr();

        tracing::debug!(
            peer_id = %self.connection.peer.id,
            addr = %addr,
            "Establishing outbound connection"
        );

        self.connection.transition_to_connecting()?;

        let stream = timeout(self.config.handshake_timeout, TcpStream::connect(addr))
            .await
            .map_err(|_| Error::ConnectionTimeout)?
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::ConnectionRefused => Error::ConnectionRefused,
                std::io::ErrorKind::ConnectionReset => Error::ConnectionReset,
                _ => Error::IOError(e),
            })?;

        self.tcp_stream = Some(stream);
        Ok(())
    }

    /// Perform handshake protocol
    async fn perform_handshake(&mut self) -> Result<()> {
        let peer_id = self.connection.peer.id;

        tracing::debug!(
            peer_id = %peer_id,
            "Starting handshake"
        );

        self.connection.transition_to_awaiting_handshake()?;

        // Create version message
        let version_msg = self.create_version_message();
        let magic = self.network.magic();

        // Send version and verack
        {
            let stream = self.tcp_stream.as_mut().ok_or(Error::Internal(
                "No TCP stream available for handshake".to_string(),
            ))?;
            Self::send_message_to_stream(
                &mut self.framer,
                stream,
                &Message::Version(version_msg),
                magic,
            )
            .await?;
            self.connection.handshake.mark_version_sent();

            Self::send_message_to_stream(&mut self.framer, stream, &Message::Verack, magic).await?;
            self.connection.handshake.mark_verack_sent();
        }

        // Wait for version and verack from peer
        let deadline = Instant::now() + self.config.handshake_timeout;

        while !self.connection.handshake.is_complete() {
            if Instant::now() > deadline {
                return Err(Error::HandshakeTimeout);
            }

            let msg = {
                let stream = self.tcp_stream.as_mut().ok_or(Error::Internal(
                    "TCP stream closed during handshake".to_string(),
                ))?;
                Self::read_message_from_stream(stream, magic).await?
            };
            self.handle_handshake_message(msg)?;
        }

        tracing::info!(
            peer_id = %peer_id,
            "Handshake complete"
        );

        let _ = self
            .control_event_tx
            .send(ControlEvent::HandshakeComplete { peer_id });

        Ok(())
    }

    /// Handle a message during handshake
    fn handle_handshake_message(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::Version(v) => {
                // Validate version message
                self.validate_version(&v)?;
                self.connection.handshake.mark_version_received(v);
            }
            Message::Verack => {
                self.connection.handshake.mark_verack_received();
            }
            _ => {
                // Ignore other messages during handshake
                tracing::debug!(
                    peer_id = %self.connection.peer.id,
                    message = ?msg,
                    "Ignoring message during handshake"
                );
            }
        }
        Ok(())
    }

    /// Validate version message
    fn validate_version(&self, version: &VersionMessage) -> Result<()> {
        // Validate network magic (implicitly validated by MessageFramer)

        // Validate user agent (check for banned patterns)
        if self.is_user_agent_banned(&version.user_agent) {
            return Err(Error::BannedUserAgent {
                user_agent: version.user_agent.clone(),
            });
        }

        // Validate blockchain type (BSV identification via user agent)
        if !self.is_bsv_user_agent(&version.user_agent) {
            return Err(Error::HandshakeFailed("Peer is not Bitcoin SV".to_string()));
        }

        Ok(())
    }

    /// Check if user agent is banned
    fn is_user_agent_banned(&self, _user_agent: &str) -> bool {
        // TODO: Add banned user agent checking when config field is added
        false
    }

    /// Check if user agent indicates BSV node
    fn is_bsv_user_agent(&self, user_agent: &str) -> bool {
        // Accept Bitcoin SV user agents
        user_agent.contains("Bitcoin SV") || user_agent.contains("BitcoinSV")
    }

    /// Create version message for handshake
    fn create_version_message(&self) -> VersionMessage {
        let peer_addr = self.connection.peer.socket_addr();

        VersionMessage {
            version: PROTOCOL_VERSION,
            services: Services::NETWORK,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            recv_addr: NetworkAddress {
                timestamp: None,
                services: Services::NETWORK,
                addr: peer_addr.ip(),
                port: peer_addr.port(),
            },
            from_addr: NetworkAddress {
                timestamp: None,
                services: Services::NETWORK,
                addr: IpAddr::V4([0, 0, 0, 0].into()),
                port: 8333,
            },
            nonce: rand::random(),
            user_agent: "/rust-bitcoinsv:0.1.0/".to_string(),
            start_height: 0,
            relay: true,
        }
    }

    /// Main message loop
    async fn message_loop(&mut self) -> Result<()> {
        let peer_id = self.connection.peer.id;
        let magic = self.network.magic();

        // Set up ping interval
        let mut ping_interval = interval(self.config.ping_interval);

        loop {
            tokio::select! {
                // Control commands
                cmd = self.control_rx.recv() => {
                    match cmd {
                        Some(PeerConnectionCommand::Disconnect) => {
                            tracing::info!(peer_id = %peer_id, "Disconnect command received");
                            break;
                        }
                        Some(PeerConnectionCommand::UpdateConfig(config)) => {
                            self.config = config;
                            tracing::debug!(peer_id = %peer_id, "Configuration updated");
                        }
                        Some(PeerConnectionCommand::SendMessage(msg)) => {
                            let stream = self.tcp_stream.as_mut().ok_or(Error::Internal(
                                "TCP stream closed".to_string(),
                            ))?;
                            Self::send_message_to_stream(&mut self.framer, stream, &msg, magic).await?;
                        }
                        None => {
                            tracing::warn!(peer_id = %peer_id, "Control channel closed");
                            break;
                        }
                    }
                }

                // Ping timer
                _ = ping_interval.tick() => {
                    let nonce = rand::random();
                    self.connection.ping_pong.record_ping(nonce);
                    let stream = self.tcp_stream.as_mut().ok_or(Error::Internal(
                        "TCP stream closed".to_string(),
                    ))?;
                    Self::send_message_to_stream(&mut self.framer, stream, &Message::Ping(nonce), magic).await?;
                }

                // Read messages
                result = async {
                    let stream = self.tcp_stream.as_mut().ok_or(Error::Internal(
                        "TCP stream closed".to_string(),
                    ))?;
                    Self::read_message_from_stream(stream, magic).await
                } => {
                    match result {
                        Ok(msg) => {
                            self.handle_message(msg, magic).await?;
                        }
                        Err(e) => {
                            tracing::error!(peer_id = %peer_id, error = %e, "Read error");
                            return Err(e);
                        }
                    }
                }
            }

            // Check ping timeouts
            let timeouts = self.connection.ping_pong.check_timeouts();
            if !timeouts.is_empty() {
                tracing::warn!(
                    peer_id = %peer_id,
                    nonce = timeouts[0],
                    "Ping timeout"
                );
                return Err(Error::PingTimeout);
            }
        }

        Ok(())
    }

    /// Handle a message in connected state
    async fn handle_message(&mut self, msg: Message, magic: u32) -> Result<()> {
        let peer_id = self.connection.peer.id;

        match &msg {
            Message::Ping(nonce) => {
                // Respond with pong
                let stream = self
                    .tcp_stream
                    .as_mut()
                    .ok_or(Error::Internal("TCP stream closed".to_string()))?;
                Self::send_message_to_stream(
                    &mut self.framer,
                    stream,
                    &Message::Pong(*nonce),
                    magic,
                )
                .await?;
            }
            Message::Pong(nonce) => {
                // Validate pong response
                match self.connection.ping_pong.validate_pong(*nonce) {
                    Ok(rtt) => {
                        tracing::debug!(
                            peer_id = %peer_id,
                            rtt_ms = rtt.as_millis(),
                            "Received pong"
                        );
                    }
                    Err(_) => {
                        tracing::warn!(
                            peer_id = %peer_id,
                            nonce = nonce,
                            "Received pong with unexpected nonce"
                        );
                    }
                }
            }
            _ => {
                // Broadcast other messages to subscribers
                let _ = self.bitcoin_message_tx.send(BitcoinMessageEvent {
                    peer_id,
                    message: msg,
                });
            }
        }

        Ok(())
    }

    /// Send a message to the peer
    async fn send_message_to_stream(
        framer: &mut MessageFramer,
        stream: &mut TcpStream,
        msg: &Message,
        magic: u32,
    ) -> Result<()> {
        let encoded = framer.frame_message(magic, msg)?;
        stream.write_all(encoded).await.map_err(Error::IOError)?;
        Ok(())
    }

    /// Read a message from the peer (simplified - reads one complete message)
    async fn read_message_from_stream(stream: &mut TcpStream, magic: u32) -> Result<Message> {
        // Read header
        let mut header_buf = [0u8; 24]; // MessageHeader::SIZE
        use tokio::io::AsyncReadExt;
        stream
            .read_exact(&mut header_buf)
            .await
            .map_err(Error::IOError)?;

        // Parse header
        let mut buf: &[u8] = &header_buf;
        let header = MessageHeader::decode(&mut buf)?;

        // Validate magic
        if header.magic != magic {
            return Err(Error::NetworkMismatch {
                expected: format!("{:08x}", magic),
                received: format!("{:08x}", header.magic),
            });
        }

        // Read payload
        let mut payload = vec![0u8; header.payload_size as usize];
        stream
            .read_exact(&mut payload)
            .await
            .map_err(Error::IOError)?;

        // Decode message based on command
        Self::decode_message(&header, &payload)
    }

    /// Decode a message from header and payload
    fn decode_message(_header: &MessageHeader, _payload: &[u8]) -> Result<Message> {
        // TODO: Implement proper message decoding
        // For now, return a placeholder to allow compilation
        // This needs to be completed with proper decoding logic for all message types
        Err(Error::Internal(
            "Message decoding not yet implemented".to_string(),
        ))
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
        let mut conn = PeerConnection::new_outbound(peer, (3, Duration::from_secs(60)), 30);

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
        let mut conn = PeerConnection::new_outbound(peer, (3, Duration::from_secs(60)), 30);

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
        let mut conn = PeerConnection::new_outbound(peer, (3, Duration::from_secs(60)), 30); // Max 3 restarts

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
        let mut conn = PeerConnection::new_outbound(peer, (2, Duration::from_secs(1)), 30); // 2 restarts, 1 sec window

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
        let conn = PeerConnection::new_outbound(peer, (3, Duration::from_secs(60)), 30);

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
        let mut conn = PeerConnection::new_outbound(peer, (3, Duration::from_secs(60)), 30);

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
        let mut conn = PeerConnection::new_outbound(peer, (3, Duration::from_secs(60)), 30);

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
