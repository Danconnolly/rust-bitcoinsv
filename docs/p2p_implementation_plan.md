# P2P Module Implementation Plan - TDD Approach

## Document Status
- **Created**: 2025-10-05
- **Based on**: p2p_design_prompt.md
- **Approach**: Test-Driven Development with early integration testing

## Design Issues Identified

### Critical Issues

1. **Connection counting race condition** (design lines 111-114, 960, 982)
   - Problem: Connection limits checked before handshake, count incremented after handshake
   - Multiple concurrent inbound connections could all pass capacity check and exceed max_connections
   - **Fix**: Atomic connection reservation system - reserve slot when creating PeerConnection, release if handshake fails

2. **Inbound reconnection strategy is flawed** (design lines 338-346)
   - Problem: Design says inbound connections should "establish new outbound connection" after network failure
   - We don't know peer's listening port (source port ≠ listening port)
   - Peer might not have a listener enabled
   - **Fix**: Inbound connections simply terminate on network failure, no reconnection attempt

3. **Missing restart tracking fields** (design line 354)
   - Problem: Design specifies max_restarts and restart_window config but no tracking fields
   - **Fix**: Add `restart_count: usize` and `restart_window_start: Timestamp` to PeerConnection state

4. **Blockchain type validation missing from VersionMessage**
   - Problem: Design requires validating "blockchain type (must be Bitcoin SV)" (lines 234, 249, 267)
   - Existing VersionMessage struct doesn't have blockchain type field
   - **Fix**: Identify BSV nodes via user agent pattern matching or protocol version

### Design Clarifications Needed

5. **Duplicate connection handling unclear** (design lines 115-118)
   - Problem: "If peer exists with status Inaccessible, that's acceptable - proceed"
   - What about Valid or Unknown status? What about existing active connections?
   - **Fix**: Check for existing active connection to same IP:port, reject duplicates

6. **DNS discovery wasteful in Fixed Peer List Mode** (design line 943)
   - Problem: "DNS discovery may execute but results are not used"
   - **Fix**: Skip DNS discovery entirely in Fixed Peer List Mode

7. **Handshake timeout start point ambiguous**
   - Problem: When does timeout timer start? At TCP connect? When sending Version?
   - **Fix**: Clarify timeout starts when entering AwaitingHandshake state

8. **Ping/Pong nonce tracking not mentioned**
   - Problem: Bitcoin protocol requires Ping nonces to be echoed in Pong
   - Design doesn't mention validating pong nonces
   - **Fix**: Track sent ping nonces, validate pong responses match

### Implementation Concerns

9. **PeerStore concurrency not specified**
   - Problem: Multiple PeerConnections update peer status concurrently
   - **Fix**: Implement PeerStore with interior mutability (Arc<Mutex<...>>)

10. **Connection count synchronization**
    - Problem: Manager needs thread-safe connection counting
    - **Fix**: Use AtomicUsize or mutex-protected counter

11. **No periodic persistence**
    - Problem: Design only persists PeerStore on shutdown (line 1013)
    - Data lost if process crashes
    - **Fix**: Add periodic background persistence (e.g., every 5 minutes)

12. **Listener restart mechanism missing**
    - Problem: Bind failures are non-fatal but no retry mechanism
    - **Fix**: Add optional periodic retry or manual restart API

13. **Over-capacity connections never counted** (design line 1004)
    - Note: This is actually acceptable - being polite to remote peer by completing handshake before rejection

14. **Network type mapping**
    - Problem: Need to map Network enum to magic values
    - **Fix**: Add Network::magic() helper method

### Minor Issues

15. **Ban reason codes vs reject codes**
    - BanReason enum is descriptive, RejectMessage uses numeric codes
    - These are different layers, acceptable as-is

16. **Status timestamp use cases vague** (design line 1030)
    - Mentions retry logic but backoff uses retry_count/last_retry_timestamp
    - Clarify: status_timestamp is for persistence/display, not retry logic

---

## Implementation Plan - TDD Approach

### Phase 0: Test Infrastructure & Remote Node Setup

**Goal**: Establish testing framework and remote node connectivity

**0.1 Set up integration test framework**
- File: `bsv/tests/p2p_integration_tests.rs`
- File: `bsv/tests/common/mod.rs`
- Create test utilities and helpers
- Set up test logging (tracing_subscriber)

**0.2 Configure remote node connection**
- File: `bsv/tests/common/config.rs`
```rust
pub fn get_test_node_address() -> SocketAddr {
    std::env::var("BSV_TEST_NODE")
        .unwrap_or_else(|_| "seed.bitcoinsv.io:8333".to_string())
        .parse()
        .expect("Invalid test node address")
}
```

**0.3 Create test message builders**
- Helper functions to build valid messages
- Mock data generators
- Test fixtures for common scenarios

**0.4 Write first integration test - basic connectivity**
```rust
#[tokio::test]
async fn test_tcp_connection_to_remote_node() {
    // Verify we can establish TCP connection to real BSV node
}
```

**Deliverables**:
- Integration test infrastructure
- Remote node connectivity verified
- Test helpers and utilities

**Estimated**: 1 day

---

### Phase 1: Core Data Structures (TDD)

**Goal**: Implement peer data structures and storage with comprehensive tests

**1.1 Write tests for Peer and PeerStatus**
- File: `bsv/src/p2p/peer.rs`
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_peer_creation_with_valid_status()

    #[test]
    fn test_peer_status_transitions()

    #[test]
    fn test_peer_serialization_deserialization()

    #[test]
    fn test_ban_reason_variants()

    #[test]
    fn test_restart_count_tracking() // NEW - fixes issue #3
}
```

**1.2 Implement Peer structures** to pass tests
- `Peer` struct: id, ip_address, port, status, status_timestamp
- `PeerStatus` enum: Valid, Inaccessible, Banned, Unknown
- `BanReason` enum: NetworkMismatch, BlockchainMismatch, BannedUserAgent
- Serialization with serde (JSON format)
- **Add restart tracking fields** (fixes issue #3)

**1.3 Write tests for PeerStore trait**
- File: `bsv/src/p2p/peer_store.rs`
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_create_peer()

    #[tokio::test]
    async fn test_find_by_ip_port()

    #[tokio::test]
    async fn test_find_by_status()

    #[tokio::test]
    async fn test_concurrent_updates() // Addresses issue #9

    #[tokio::test]
    async fn test_duplicate_prevention()

    #[tokio::test]
    async fn test_update_peer_status()

    #[tokio::test]
    async fn test_count_by_status()
}
```

**1.4 Implement InMemoryPeerStore** to pass tests
- Thread-safe with Arc<Mutex<...>> (fixes issue #9)
- HashMap<Uuid, Peer> for primary storage
- HashMap<(IpAddr, u16), Uuid> for IP:port index
- File persistence (load/save JSON)
- Handle corrupted/missing files gracefully
- Periodic persistence every 5 minutes (fixes issue #11)

**1.5 Write integration test for PeerStore persistence**
```rust
#[tokio::test]
async fn test_peer_store_survives_restart()
```

**1.6 Add P2P error types**
- File: `bsv/src/result.rs`
- Extend Error enum with P2P-specific variants
- Write tests for error classification (retryable vs non-retryable)
- Include all error types from design document section "Error Types"

**Deliverables**:
- Peer, PeerStatus, BanReason with tests
- PeerStore trait and InMemoryPeerStore with tests
- P2P error types with classification
- Persistence verified via integration test

**Estimated**: 2 days

---

### Phase 2: Configuration & Network Types (TDD)

**Goal**: Implement configuration structures and validation

**2.1 Write tests for Network and configuration**
- File: `bsv/src/p2p/config.rs`
```rust
#[test]
fn test_network_magic_values() // Fixes issue #14
#[test]
fn test_config_validation_rejects_invalid_limits()
#[test]
fn test_config_defaults()
#[test]
fn test_blockchain_identification_from_user_agent() // Addresses issue #4
#[test]
fn test_target_must_not_exceed_max()
```

**2.2 Implement configuration structures**
- `Network` enum: Mainnet, Testnet, Regtest
- `Network::magic(&self) -> u32` helper (fixes issue #14)
- `ManagerConfig` with manager-level settings (from design section 19)
- `ConnectionConfig` with connection-level settings (from design section 19)
- Validation: ensure target_connections <= max_connections
- Default implementations with sensible values
- BSV identification strategy via user agent (fixes issue #4)

**2.3 Integration test - verify magic with remote node**
```rust
#[tokio::test]
async fn test_remote_node_responds_to_correct_magic()
```

**Deliverables**:
- Network enum with magic values
- ManagerConfig and ConnectionConfig with validation
- BSV node identification strategy
- Verified magic values work with remote node

**Estimated**: 1 day

---

### Phase 3: Message Handling & Handshake (TDD + Integration)

**Goal**: Implement handshake protocol with validation, test against real nodes

**3.1 Write tests for handshake logic**
- File: `bsv/src/p2p/handshake.rs` or in connection.rs
```rust
#[test]
fn test_handshake_state_transitions()

#[test]
fn test_handshake_validation_rejects_wrong_network()

#[test]
fn test_handshake_validation_rejects_banned_user_agent()

#[test]
fn test_handshake_timeout_starts_at_awaiting_handshake() // Fixes issue #7

#[test]
fn test_handshake_handles_messages_in_any_order()

#[test]
fn test_handshake_four_flags_all_required()
```

**3.2 Integration test - handshake with remote node**
```rust
#[tokio::test]
async fn test_complete_handshake_with_remote_node() {
    // 1. Connect to remote node via TCP
    // 2. Send Version message
    // 3. Receive Version from peer, validate
    // 4. Exchange Verack messages
    // 5. Verify handshake complete (all 4 flags true)
}

#[tokio::test]
async fn test_receive_version_from_remote_node()

#[tokio::test]
async fn test_sendheaders_after_handshake()
```

**3.3 Implement handshake protocol**
- HandshakeState struct with 4 boolean flags:
  - version_sent, version_received, verack_sent, verack_received
- Handshake validation:
  - Network validation (matches our configuration)
  - Blockchain validation via user agent (fixes issue #4)
  - User agent ban checking
- Timeout handling (timer starts at AwaitingHandshake - fixes issue #7)
- Support messages arriving in any order
- Separate flows for outbound/inbound/over-capacity

**3.4 Write tests for ping/pong**
```rust
#[test]
fn test_ping_nonce_generation()

#[test]
fn test_ping_nonce_tracking() // Fixes issue #8

#[test]
fn test_pong_validation_matches_nonce() // Fixes issue #8

#[test]
fn test_ping_timeout()

#[test]
fn test_respond_to_received_ping()
```

**3.5 Integration test - ping/pong with remote node**
```rust
#[tokio::test]
async fn test_ping_pong_exchange_with_remote_node() {
    // Connect, complete handshake, then ping/pong exchange
}
```

**3.6 Implement ping/pong keepalive**
- Periodic ping sender (configurable interval)
- **Nonce tracking: HashMap<u64, Instant>** (fixes issue #8)
- Pong validation against sent nonces (fixes issue #8)
- Pong responder (echo nonce from received ping)
- Timeout handling

**Deliverables**:
- Handshake protocol fully implemented
- Network, blockchain, user agent validation
- Ping/pong with nonce tracking
- All tests pass including remote node integration tests

**Estimated**: 3-4 days

---

### Phase 4: Connection Actor (TDD + Integration)

**Goal**: Implement full connection state machine and lifecycle

**4.1 Write tests for connection state machine**
- File: `bsv/src/p2p/connection.rs`
```rust
#[test]
fn test_outbound_connection_state_transitions()

#[test]
fn test_inbound_connection_state_transitions()

#[test]
fn test_over_capacity_rejection_flow()

#[test]
fn test_connection_restart_after_network_failure()

#[test]
fn test_restart_limit_enforcement()

#[test]
fn test_inbound_does_not_reconnect() // Fixes issue #2

#[test]
fn test_restart_count_reset_after_window()

#[test]
fn test_network_vs_non_network_error_handling()
```

**4.2 Integration test - full connection lifecycle**
```rust
#[tokio::test]
async fn test_outbound_connection_lifecycle_with_remote_node() {
    // Connect -> Handshake -> Connected -> Messages -> Disconnect
}

#[tokio::test]
async fn test_connection_survives_network_interruption() {
    // Test restart logic (if feasible with remote node)
}
```

**4.3 Implement PeerConnection actor**
- ConnectionState enum: Disconnected, Connecting, AwaitingHandshake, Connected, Rejected, Failed
- PeerConnection struct fields:
  - Peer info
  - TCP stream (TcpStream with split read/write)
  - State
  - HandshakeState
  - send_headers_mode: bool
  - retry_count: usize, last_retry_timestamp: Option<Instant>
  - **restart_count: usize, restart_window_start: Option<Instant>** (NEW - fixes issue #3)
  - is_inbound: bool
  - reject_after_handshake: bool
  - Control channel receiver (mpsc)
  - Event broadcast senders (for Manager)
  - ping_nonces: HashMap<u64, Instant>
- State machine with transitions from design section 5
- **Inbound connections terminate on network failure** (fixes issue #2)
- Restart tracking and limits (fixes issue #3)
- Message reading loop using MessageFramer
- Message dispatch based on state
- Control command processing

**4.4 Write tests for backoff strategy**
```rust
#[test]
fn test_exponential_backoff_calculation()

#[test]
fn test_max_retries_enforcement()

#[test]
fn test_successful_connection_resets_retry_count()

#[test]
fn test_retryable_vs_non_retryable_errors()
```

**4.5 Implement reconnection backoff**
- Exponential backoff: initial_backoff * (multiplier ^ retry_count)
- Track retry attempts
- Mark as Inaccessible after max_retries
- Reset count on successful connection
- Distinguish retryable errors from ban-worthy errors

**Deliverables**:
- Full PeerConnection actor implementation
- All state transitions working
- Restart logic with tracking and limits
- Backoff strategy implemented
- Integration tests pass with remote node

**Estimated**: 5-6 days

---

### Phase 5: Manager & Connection Management (TDD + Integration)

**Goal**: Implement Manager actor with connection orchestration

**5.1 Write tests for atomic connection counting**
- File: `bsv/src/p2p/manager.rs`
```rust
#[tokio::test]
async fn test_connection_count_reservation() // Fixes issue #1

#[tokio::test]
async fn test_concurrent_inbound_connections_respect_max() // Fixes issue #1

#[tokio::test]
async fn test_connection_released_on_handshake_failure()

#[tokio::test]
async fn test_connection_count_accurate_under_concurrent_load()
```

**5.2 Write tests for connection initiation**
```rust
#[tokio::test]
async fn test_normal_mode_reaches_target_connections()

#[tokio::test]
async fn test_normal_mode_respects_max_connections()

#[tokio::test]
async fn test_fixed_peer_list_mode_only_connects_to_list()

#[tokio::test]
async fn test_skips_banned_peers()

#[tokio::test]
async fn test_skips_inaccessible_peers()

#[tokio::test]
async fn test_prioritizes_valid_over_unknown_peers()
```

**5.3 Implement Manager structure**
- Fields:
  - config: ManagerConfig
  - peer_store: Arc<dyn PeerStore>
  - **connection_slots: Arc<AtomicUsize>** (for reservation - fixes issue #1)
  - active_connections: HashMap<Uuid, PeerConnectionHandle>
  - control_event_tx: broadcast::Sender<ControlEvent>
  - bitcoin_message_tx: broadcast::Sender<BitcoinMessageEvent>
  - operating_mode: OperatingMode (Normal or FixedPeerList)
  - fixed_peers: Option<Vec<Peer>>
  - shutdown_tx: watch::Sender<bool>
- Atomic connection reservation system (fixes issue #1):
  - Reserve slot before creating PeerConnection
  - Release slot if handshake fails
  - Increment on handshake success
  - Decrement on connection close

**5.4 Write tests for duplicate connection prevention**
```rust
#[tokio::test]
async fn test_prevents_duplicate_outbound_connections()

#[tokio::test]
async fn test_prevents_duplicate_inbound_connections() // Fixes issue #5

#[tokio::test]
async fn test_handles_simultaneous_bidirectional_connections()

#[tokio::test]
async fn test_rejects_duplicate_active_connection() // Fixes issue #5
```

**5.5 Implement connection initiation logic**
- Normal mode:
  - Query PeerStore for Valid peers, then Unknown
  - Skip Banned and Inaccessible
  - **Check for existing active connection before initiating** (fixes issue #5)
  - Initiate until total_connections reaches target_connections
  - Continuous monitoring to maintain target
- Fixed peer list mode:
  - Only connect to peers in provided list
  - Skip banned peers
  - Still respect max_connections for inbound

**5.6 Write tests for inbound listener**
```rust
#[tokio::test]
async fn test_listener_binds_successfully()

#[tokio::test]
async fn test_listener_bind_failure_is_non_fatal() // Per design

#[tokio::test]
async fn test_listener_rejects_banned_ip() // Fixes issue #5

#[tokio::test]
async fn test_listener_rejects_duplicate_active_connection() // Fixes issue #5

#[tokio::test]
async fn test_listener_creates_peer_with_reject_flag_when_over_capacity()

#[tokio::test]
async fn test_listener_accepts_when_under_capacity()
```

**5.7 Integration test - inbound connections**
```rust
#[tokio::test]
async fn test_accept_inbound_connection() {
    // Start listener, simulate inbound connection
    // Or create second Manager instance to connect
}
```

**5.8 Implement inbound connection listener**
- Optional based on enable_listener config
- Bind to listener_address:listener_port
- **Bind failures are non-fatal** - log error, continue with outbound only
- For each accepted connection:
  - Extract peer IP:port
  - **Check if IP is banned using find_by_ip_port()** (fixes issue #5)
  - If banned, drop connection immediately (log at DEBUG)
  - **Check for existing active connection to same IP:port** (fixes issue #5)
  - If duplicate, drop connection
  - Check connection count
  - If >= max_connections: create with reject_after_handshake=true
  - If < max_connections: create normally
  - Create PeerConnection actor with established TCP stream

**5.9 Write tests for event handling**
```rust
#[tokio::test]
async fn test_manager_updates_peer_status_on_ban()

#[tokio::test]
async fn test_manager_initiates_new_connection_when_below_target()

#[tokio::test]
async fn test_manager_broadcasts_control_events()

#[tokio::test]
async fn test_manager_broadcasts_bitcoin_messages()

#[tokio::test]
async fn test_manager_handles_connection_failed_event()
```

**5.10 Implement event handling**
- Listen to ControlEvents from PeerConnections
- Update peer status in PeerStore based on events
- Handle ConnectionEstablished, ConnectionFailed, PeerBanned, etc.
- Re-initiate connections if below target (Normal mode)
- Broadcast events to external subscribers

**5.11 Write tests for configuration updates**
```rust
#[tokio::test]
async fn test_config_update_propagates_to_connections()

#[tokio::test]
async fn test_invalid_config_update_rejected()

#[tokio::test]
async fn test_listener_restart_on_address_change() // Addresses issue #12

#[tokio::test]
async fn test_target_connections_update_triggers_new_connections()
```

**5.12 Implement dynamic configuration updates**
- Validate new configuration
- Apply to Manager state
- Propagate ConnectionConfig to active PeerConnections
- Handle listener restart if address/port changed (addresses issue #12)
- Update connection targets and initiate/close as needed

**Deliverables**:
- Full Manager implementation
- Atomic connection counting preventing race conditions
- Duplicate connection prevention
- Inbound listener with all safety checks
- Event handling and peer status updates
- Dynamic configuration updates
- All tests pass including integration tests

**Estimated**: 5-6 days

---

### Phase 6: DNS Discovery (TDD + Integration)

**Goal**: Implement DNS-based peer discovery

**6.1 Write tests for DNS discovery**
- File: `bsv/src/p2p/discovery.rs`
```rust
#[tokio::test]
async fn test_dns_discovery_adds_unknown_peers()

#[tokio::test]
async fn test_dns_discovery_skips_banned_ips()

#[tokio::test]
async fn test_dns_discovery_handles_resolution_failure()

#[tokio::test]
async fn test_dns_discovery_skipped_in_fixed_mode() // Fixes issue #6

#[tokio::test]
async fn test_dns_discovery_skips_duplicates()

#[tokio::test]
async fn test_dns_discovery_uses_default_port()
```

**6.2 Integration test - real DNS discovery**
```rust
#[tokio::test]
async fn test_discover_peers_from_real_dns_seeds() {
    // Use actual BSV DNS seeds
    // Verify peers are added to store
}
```

**6.3 Implement DNS discovery**
- Async DNS resolution using tokio::net::lookup_host or trust-dns
- Query each DNS seed from configuration
- **Only run in Normal Mode** (skip entirely in Fixed Peer List Mode - fixes issue #6)
- For each resolved IP:
  - Check if IP:port already exists using find_by_ip_port()
  - **Skip if peer is banned** (addresses issue #6)
  - Create new Unknown peer if not duplicate
  - Use default_port from configuration
- Scheduled execution:
  - Run at startup (Normal mode only)
  - Run periodically (every hour) during operation
- Handle DNS failures gracefully (log warning, continue)

**Deliverables**:
- DNS discovery implementation
- Integration with PeerStore
- Scheduled execution (startup + hourly)
- Skipped in Fixed Peer List Mode
- Integration test with real DNS seeds passes

**Estimated**: 2 days

---

### Phase 7: Internal Messages & Channels (TDD)

**Goal**: Define and implement internal message passing

**7.1 Write tests for message types**
- File: `bsv/src/p2p/messages.rs`
```rust
#[test]
fn test_peer_connection_command_variants()

#[test]
fn test_control_event_variants()

#[test]
fn test_ban_reason_serialization()

#[test]
fn test_bitcoin_message_event_construction()
```

**7.2 Implement internal message types**
- PeerConnectionCommand enum (from design section "Internal Message Types"):
  - UpdateConfig(ConnectionConfig)
  - Disconnect
  - SendMessage(Message)
- ControlEvent enum (from design):
  - ConnectionEstablished, ConnectionFailed, ConnectionLost, etc.
  - All variants from design section
- BanReason enum (already defined in Phase 1)
- BitcoinMessageEvent struct:
  - peer_id: Uuid
  - message: Message

**7.3 Write tests for broadcast channels**
```rust
#[tokio::test]
async fn test_multiple_subscribers_receive_events()

#[tokio::test]
async fn test_channel_overflow_handling()

#[tokio::test]
async fn test_late_subscriber_doesnt_receive_old_events()

#[tokio::test]
async fn test_channel_capacity_configuration()
```

**7.4 Implement broadcast channel setup**
- Create broadcast channels in Manager
- Appropriate capacity (e.g., 1000 events)
- Subscription interface for external consumers

**Deliverables**:
- All internal message types defined
- Broadcast channels implemented
- Tests verify channel behavior

**Estimated**: 1 day

---

### Phase 8: Observability (TDD)

**Goal**: Add logging and telemetry support

**8.1 Write tests for logging**
- File: Tests in relevant modules
```rust
#[test]
fn test_structured_logging_includes_peer_context()

#[test]
fn test_log_level_filtering_works()

#[test]
fn test_connection_events_logged_at_info()

#[test]
fn test_errors_logged_at_error_level()
```

**8.2 Implement tracing integration**
- Add tracing instrumentation to all key operations
- Structured fields: peer_id, peer_address, connection_type, state, error
- Appropriate log levels (design section 13):
  - ERROR: Critical failures, bind failures
  - WARN: Connection failures, retry attempts
  - INFO: Connections established/lost, handshakes complete
  - DEBUG: State transitions, peer discovery
  - TRACE: Message contents, detailed debugging
- Instrument Manager, PeerConnection, DNS discovery

**8.3 Write tests for telemetry**
```rust
#[test]
fn test_telemetry_metrics_recorded()

#[test]
fn test_telemetry_can_be_disabled()

#[test]
fn test_connection_count_gauge_updates()

#[test]
fn test_handshake_duration_histogram()
```

**8.4 Implement OpenTelemetry support (optional)**
- Optional feature flag: "telemetry"
- Metrics from design section 13:
  - Connection count (gauge)
  - Connections by status (gauge)
  - Connection attempts (counter)
  - Connection failures by reason (counter)
  - Handshake duration (histogram)
  - Messages sent/received by type (counter)
  - Ban events (counter)
  - Restart events (counter)
- Traces: spans for connection lifecycle, handshake, message exchange
- Events: key lifecycle events
- Configurable endpoint and service name

**Deliverables**:
- Comprehensive tracing instrumentation
- Optional OpenTelemetry metrics and traces
- All events properly logged
- Tests verify logging behavior

**Estimated**: 2 days

---

### Phase 9: Public API & Integration (TDD + Integration)

**Goal**: Finalize public API and comprehensive integration testing

**9.1 Write tests for Manager API**
- File: `bsv/src/p2p/manager.rs`
```rust
#[tokio::test]
async fn test_manager_new_normal_mode()

#[tokio::test]
async fn test_manager_with_fixed_peers()

#[tokio::test]
async fn test_manager_start_and_shutdown()

#[tokio::test]
async fn test_manager_send_message_to_peer()

#[tokio::test]
async fn test_manager_ban_peer()

#[tokio::test]
async fn test_manager_unban_peer()

#[tokio::test]
async fn test_manager_subscribe_control_events()

#[tokio::test]
async fn test_manager_subscribe_bitcoin_messages()

#[tokio::test]
async fn test_manager_get_connection_count()

#[tokio::test]
async fn test_manager_get_peer_list()
```

**9.2 Integration test - full system with remote node**
- File: `bsv/tests/p2p_integration_tests.rs`
```rust
#[tokio::test]
async fn test_full_manager_lifecycle_with_remote_node() {
    // 1. Create Manager with config
    // 2. Start Manager
    // 3. Subscribe to control events
    // 4. Wait for connection to remote node
    // 5. Verify handshake complete event
    // 6. Subscribe to bitcoin messages
    // 7. Send GetAddr message
    // 8. Receive Addr response
    // 9. Shutdown gracefully
    // 10. Verify PeerStore persisted
}

#[tokio::test]
async fn test_manager_maintains_target_connections()

#[tokio::test]
async fn test_manager_handles_peer_bans()

#[tokio::test]
async fn test_manager_config_updates_apply()

#[tokio::test]
async fn test_manager_connection_limits_enforced()
```

**9.3 Implement Manager public API**
- File: `bsv/src/p2p/manager.rs`
```rust
impl Manager {
    pub fn new(config: ManagerConfig, peer_store: Arc<dyn PeerStore>) -> Self

    pub fn with_fixed_peers(
        config: ManagerConfig,
        peer_store: Arc<dyn PeerStore>,
        peers: Vec<Peer>
    ) -> Self

    pub async fn start(&mut self) -> Result<()>

    pub async fn shutdown(&mut self) -> Result<()>

    pub async fn update_config(&mut self, config: ManagerConfig) -> Result<()>

    pub fn subscribe_control_events(&self) -> broadcast::Receiver<ControlEvent>

    pub fn subscribe_bitcoin_messages(&self) -> broadcast::Receiver<BitcoinMessageEvent>

    pub async fn send_message(&self, peer_id: Uuid, message: Message) -> Result<()>

    pub async fn ban_peer(&mut self, peer_id: Uuid) -> Result<()>

    pub async fn unban_peer(&mut self, peer_id: Uuid) -> Result<()>

    pub fn get_connection_count(&self) -> usize

    pub async fn get_peers(&self) -> Result<Vec<Peer>>
}
```

**9.4 Module organization**
- File: `bsv/src/p2p/mod.rs`
```rust
mod config;
mod connection;
mod discovery;
mod handshake;
mod manager;
mod messages;
mod peer;
mod peer_store;

pub use config::{ConnectionConfig, ManagerConfig, Network};
pub use manager::Manager;
pub use messages::{BanReason, BitcoinMessageEvent, ControlEvent, PeerConnectionCommand};
pub use peer::{Peer, PeerStatus};
pub use peer_store::{InMemoryPeerStore, PeerStore};

// Re-export from existing p2p module
pub use super::p2p::{Message, VersionMessage, /* etc */};
```

**Deliverables**:
- Complete Manager public API
- Clean module organization with re-exports
- Comprehensive integration tests with remote node
- All API methods tested

**Estimated**: 2-3 days

---

### Phase 10: End-to-End Testing & Refinement

**Goal**: Comprehensive testing and performance validation

**10.1 Write comprehensive integration tests**
- File: `bsv/tests/p2p_integration_tests.rs`
```rust
#[tokio::test]
async fn test_network_partition_and_recovery()

#[tokio::test]
async fn test_many_concurrent_connections()

#[tokio::test]
async fn test_peer_store_persistence_across_restarts()

#[tokio::test]
async fn test_fixed_peer_list_mode_complete()

#[tokio::test]
async fn test_connection_limit_enforcement_under_load()

#[tokio::test]
async fn test_dns_discovery_integration()

#[tokio::test]
async fn test_ban_enforcement_across_manager_restart()

#[tokio::test]
async fn test_configuration_hot_reload()

#[tokio::test]
async fn test_graceful_shutdown_with_active_connections()
```

**10.2 Performance testing**
- File: `bsv/benches/p2p_benchmarks.rs`
- Benchmark connection establishment time
- Benchmark message throughput (messages/sec)
- Benchmark memory usage with many connections
- Stress test connection limits (e.g., 100 concurrent connections)
- Profile with cargo flamegraph

**10.3 Chaos testing**
- Implement chaos testing utilities
- Random network failures during operation
- Random peer disconnections
- Concurrent configuration updates
- Memory leak detection with valgrind or similar

**10.4 Bug fixes and refinement**
- Address issues found during comprehensive testing
- Performance optimizations
- Code cleanup and refactoring
- Documentation improvements

**Deliverables**:
- Comprehensive test suite passes
- Performance benchmarks established
- No memory leaks detected
- Code is production-ready quality

**Estimated**: 2-3 days

---

### Phase 11: Documentation & Examples

**Goal**: Complete documentation and usage examples

**11.1 Write API documentation**
- Add rustdoc comments to all public items
- Usage examples in doc comments
- Module-level documentation
- Configuration guide
- Common patterns and best practices

**11.2 Create example programs**
- File: `examples/simple_p2p_client.rs`
```rust
// Connect to BSV network, subscribe to events
// Send GetAddr, receive addresses
// Demonstrate basic usage
```

- File: `examples/fixed_peer_connection.rs`
```rust
// Connect to specific trusted nodes
// Demonstrate fixed peer list mode
```

- File: `examples/event_monitor.rs`
```rust
// Monitor P2P network events
// Log all control events and messages
// Demonstrate event subscription
```

- File: `examples/network_crawler.rs`
```rust
// Discover and connect to many peers
// Collect network topology information
```

**11.3 Update project documentation**
- Update `CLAUDE.md` with P2P module information
- Add P2P section to main README if appropriate
- Create `docs/p2p_usage_guide.md` with:
  - Getting started
  - Configuration options explained
  - Common patterns
  - Troubleshooting
  - FAQ

**Deliverables**:
- Comprehensive rustdoc documentation
- 4+ working example programs
- Updated project documentation
- Usage guide for module consumers

**Estimated**: 2 days

---

## Implementation Workflow

### TDD Cycle (Red-Green-Refactor)

For each component:

1. **Write failing test** - Define expected behavior
2. **Run test** - Verify it fails for the right reason (Red)
3. **Implement minimum code** - Make test pass
4. **Run test** - Verify it passes (Green)
5. **Refactor** - Improve code quality while keeping tests green
6. **Run all tests** - Verify no regressions
7. **Repeat** for next test case

### Integration Test Strategy

**Environment Setup**:
```bash
# Use default BSV testnet node
cargo test --test p2p_integration_tests

# Use specific node
BSV_TEST_NODE=192.168.1.100:8333 cargo test --test p2p_integration_tests

# Skip integration tests (unit tests only)
cargo test --lib
```

**Test Configuration** (`bsv/tests/common/config.rs`):
```rust
pub fn get_test_node_address() -> SocketAddr {
    std::env::var("BSV_TEST_NODE")
        .unwrap_or_else(|_| "seed.bitcoinsv.io:8333".to_string())
        .parse()
        .expect("Invalid test node address")
}

pub fn get_test_network() -> Network {
    std::env::var("BSV_TEST_NETWORK")
        .unwrap_or_else(|_| "mainnet".to_string())
        .parse()
        .expect("Invalid network")
}
```

### Continuous Integration

Each phase should maintain:
- ✅ All unit tests passing
- ✅ All integration tests passing
- ✅ No compiler warnings
- ✅ cargo clippy clean
- ✅ cargo fmt applied

---

## Context Management Strategy

Given context size concerns, recommended approach:

### Checkpoint Documents

After completing each major phase group, create checkpoint:
- File: `docs/p2p_checkpoint_N.md`
- Contains:
  - What was implemented
  - Key design decisions
  - Test coverage summary
  - Known issues or TODOs
  - API surface area

### Phase Groupings for Context Management

**Batch 1**: Foundation (Phases 0-2)
- Test infrastructure
- Core data structures
- Configuration
- **Checkpoint**: `docs/p2p_checkpoint_1_foundation.md`

**Batch 2**: Protocol Implementation (Phases 3-4)
- Handshake and validation
- Connection actor and state machine
- **Checkpoint**: `docs/p2p_checkpoint_2_protocol.md`

**Batch 3**: Orchestration (Phases 5-6)
- Manager implementation
- DNS discovery
- **Checkpoint**: `docs/p2p_checkpoint_3_orchestration.md`

**Batch 4**: Completion (Phases 7-11)
- Messages and channels
- Observability
- Public API
- Testing and documentation
- **Checkpoint**: `docs/p2p_checkpoint_4_completion.md`

### Between Batches

1. Create checkpoint document summarizing batch
2. Commit all code
3. Start fresh context for next batch
4. Reference checkpoint and this plan document
5. Continue with next batch

---

## Estimated Timeline

### With TDD Approach

- **Phase 0**: Test infrastructure - 1 day
- **Phase 1**: Core structures (TDD) - 2 days
- **Phase 2**: Config (TDD) - 1 day
- **Phase 3**: Handshake (TDD + Integration) - 3-4 days
- **Phase 4**: Connection Actor (TDD + Integration) - 5-6 days
- **Phase 5**: Manager (TDD + Integration) - 5-6 days
- **Phase 6**: DNS Discovery (TDD + Integration) - 2 days
- **Phase 7**: Messages (TDD) - 1 day
- **Phase 8**: Observability (TDD) - 2 days
- **Phase 9**: Public API (TDD + Integration) - 2-3 days
- **Phase 10**: E2E Testing - 2-3 days
- **Phase 11**: Documentation - 2 days

**Total**: ~4500 LOC (including tests), 26-33 days

### Code Distribution Estimate

- **Implementation**: ~2200 LOC
- **Unit tests**: ~1500 LOC
- **Integration tests**: ~600 LOC
- **Documentation**: ~200 LOC

### Benefits of TDD Approach

✅ Higher code quality from day one
✅ Better protocol compliance (tested against real nodes early)
✅ Fewer bugs in production
✅ Living documentation through tests
✅ Confidence in refactoring
✅ Catches integration issues early
✅ Easier debugging (isolated test cases)

The TDD approach adds ~25% more time but delivers significantly higher quality code with comprehensive test coverage from the start.

---

## Success Criteria

### Phase Completion Checklist

Each phase is complete when:
- [ ] All unit tests pass
- [ ] All integration tests pass (if applicable)
- [ ] No compiler warnings
- [ ] cargo clippy clean
- [ ] cargo fmt applied
- [ ] Code reviewed and refactored
- [ ] Documentation updated
- [ ] Checkpoint document created (if end of batch)

### Module Completion Criteria

The P2P module is complete when:
- [ ] All 11 phases finished
- [ ] Full test suite passes (unit + integration)
- [ ] Can successfully connect to real BSV nodes
- [ ] Can complete handshake with real nodes
- [ ] Can send/receive messages with real nodes
- [ ] Connection limits enforced correctly
- [ ] Banning works correctly
- [ ] DNS discovery finds real peers
- [ ] Configuration updates work dynamically
- [ ] Graceful shutdown works
- [ ] PeerStore persists correctly
- [ ] No memory leaks
- [ ] Performance benchmarks acceptable
- [ ] API documentation complete
- [ ] Example programs work
- [ ] All design issues resolved

---

## Next Steps

To begin implementation:

1. **Set up test infrastructure** (Phase 0)
   - Create integration test directory
   - Configure remote node connectivity
   - Write first connectivity test
   - Verify test passes with real node

2. **Start Phase 1** (Core Data Structures)
   - Write tests for Peer struct
   - Implement Peer to pass tests
   - Continue with TDD cycle

3. **Maintain checkpoint documents** as you progress through batches

4. **Use fresh context between batches** as needed for token efficiency

---

*Last updated: 2025-10-05*
