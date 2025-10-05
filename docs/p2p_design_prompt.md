# P2P Connection Manager Module - Software Design Document

## Overview
A new module for an existing Rust library that implements peer-to-peer communication between Bitcoin SV nodes. The module uses the actor model pattern with Tokio async runtime for managing multiple peer connections.

## Technology Stack
- **Language**: Rust
- **Async Runtime**: Tokio
- **Architecture Pattern**: Actor Model
- **Network Protocol**: Bitcoin SV P2P Protocol
- **Transport**: TCP
- **Logging**: Standard Rust logging (`tracing` crate recommended)
- **Observability**: OpenTelemetry support

## Core Components

### 1. Manager (Top-Level Actor)
The `Manager` is the highest-level object that orchestrates all peer connections.

**Responsibilities:**
- Manage the list of peers via a `PeerStore` implementation
- Create and supervise `PeerConnection` objects
- Control connections via Tokio channels
- **Maintain connection count between target and maximum limits**
- Provide broadcast channels for events
- Accept and process dynamic configuration updates
- Propagate configuration changes to active connections and channels
- Initiate and manage DNS Peer Discovery process
- Set up and manage inbound connection listener (when enabled)
- Create PeerConnection actors for inbound connections
- Enforce ban rules on inbound connections
- **Reject excess inbound connections when above maximum**
- **Handle listener failures gracefully (non-fatal)**

**Broadcast Channels:**
The Manager creates two broadcast channels accessible to module users:
1. **Control Messages Channel**: Broadcasts control/administrative messages received from peers
2. **Bitcoin Messages Channel**: Broadcasts Bitcoin protocol messages received from peers

These channels allow external consumers to receive P2P events and react accordingly.

### 2. Connection Management Strategy

The Manager maintains connection counts within configurable limits.

**Connection Limits (configurable):**
- **`target_connections`**: Target number of total connections (default: 8)
- **`max_connections`**: Maximum number of total connections (default: 20)

**Connection Count Rules:**
- **Total connections** = outbound connections + inbound connections
- **Below target**: Manager actively initiates new outbound connections
- **Between target and max**: Manager accepts inbound connections but doesn't initiate new outbound connections
- **At or above max**: Manager rejects new inbound connections after handshake using Reject message

**Connection Initiation Behavior:**

**Normal Mode:**
- While `total_connections < target_connections`:
  - Initiate outbound connections to peers from the store
  - Priority: `Valid` peers first, then `Unknown` peers
  - Skip `Banned` and `Inaccessible` peers
- When `total_connections >= target_connections`:
  - Stop initiating new outbound connections
  - Continue accepting inbound connections (up to max)
- Continuously monitor connection count and initiate new connections if count drops below target

**Fixed Peer List Mode:**
- Ignores `target_connections` for outbound connection decisions
- Only connects to peers in the fixed list (when not banned)
- Still respects `max_connections` for inbound connection acceptance

**Inbound Connection Acceptance:**
- Always check if IP is banned first (reject immediately if banned)
- If `total_connections < max_connections`:
  - Accept the connection and proceed with handshake
- If `total_connections >= max_connections`:
  - Accept the TCP connection
  - Proceed with handshake (to be protocol-compliant)
  - After successful handshake, send `Reject` message
  - Close the connection
  - Do not count as an active connection

### 3. Inbound Connection Listener

The Manager can optionally listen for inbound peer connections.

**Listener Characteristics:**
- **Optional**: Listener is only active when enabled in configuration
- **Configurable bind address and port**: Specified in configuration
- **Non-blocking**: Runs as a separate async task
- **Peer creation**: When a peer connects, Manager creates a new `PeerConnection` actor to handle it
- **Ban enforcement**: Checks if incoming IP address is banned before accepting connection
- **Capacity enforcement**: Checks connection count limits
- **Non-fatal failures**: Listener bind failures do not prevent Manager from starting

**Listener Behavior:**
1. Attempt to bind to configured address and port
2. **If bind fails:**
   - Log error at ERROR level
   - Emit OpenTelemetry event
   - Continue Manager startup (outbound connections still work)
   - Listener remains disabled
3. **If bind succeeds:**
   - Accept incoming TCP connections
   - For each accepted connection:
     - Extract peer IP address and port from socket
     - **Check if IP address is banned using `find_by_ip_port()` and checking status**
     - **If banned, immediately drop connection without creating PeerConnection**
     - Check current total connection count
     - **If `total_connections >= max_connections`:**
       - Create PeerConnection actor with `reject_after_handshake` flag set
       - PeerConnection will complete handshake then send Reject message and disconnect
     - **If `total_connections < max_connections`:**
       - Check if peer already exists in store using `find_by_ip_port()`
       - If peer doesn't exist, create new peer entry with status `Unknown`
       - **If peer exists with status `Inaccessible`, that's acceptable - proceed with connection**
       - Create new `PeerConnection` actor with the established TCP connection
       - Hand off connection to the PeerConnection actor
       - PeerConnection immediately begins handshake process (expects Version from peer)

**Connection Limits:**
- In Normal Mode: Inbound connections count toward total connection count
- In Fixed Peer List Mode: Inbound connections count toward total connection count
- `max_connections` applies to total connections (inbound + outbound)

**Error Handling:**
- **Listener bind failures: Non-fatal, log error and continue with outbound connections**
- Individual connection acceptance failures should not crash the listener (log at WARN level)
- Invalid or malformed connections should be rejected gracefully
- **Banned connections are dropped silently (log at DEBUG level)**
- **Excess connections (over max) are rejected with Reject message after handshake**

### 4. PeerConnection (Per-Peer Actor)
Each `PeerConnection` object represents a connection to a specific peer.

**Connection Model:**
- **Initial Implementation**: Uses a single TCP connection channel per peer
- **Bidirectional Communication**: The channel must support simultaneous send and receive operations
- **Connection Origin**: Can be either outbound (initiated by us) or inbound (initiated by peer)
- **Future Enhancement**: Will support up to 2 channels (second channel to be added later)
- Each channel is a separate TCP connection

**Connection State:**
- **`state`**: Current connection state (see Connection State Machine below)
- **`send_headers_mode`**: Boolean flag indicating if peer prefers headers over block announcements
- **`retry_count`**: Number of connection retry attempts for this peer (outbound only)
- **`last_retry_timestamp`**: Timestamp of last retry attempt (outbound only)
- **`is_inbound`**: Boolean flag indicating if this is an inbound connection
- **`reject_after_handshake`**: Boolean flag indicating connection should be rejected after handshake (capacity limit)

**Responsibilities:**
- Maintain TCP connection to a specific peer
- Execute connection handshake protocol
- Validate peer during handshake (network, blockchain, user agent)
- **Send Reject message and disconnect if reject_after_handshake flag is set**
- Send events to Manager's broadcast channels
- Receive control messages from Manager via Tokio channels
- Handle Bitcoin protocol message exchange (using existing Message enum from p2p module)
- Apply configuration updates received from Manager
- Manage periodic ping/pong keepalive mechanism
- Implement reconnection backoff strategy (for outbound connections)
- **Automatically restart connection after network-level failures (for established connections)**

### 5. Connection State Machine

The connection follows an explicit state machine to manage its lifecycle.

**States:**
- **`Disconnected`**: No active connection (outbound only)
- **`Connecting`**: TCP connection attempt in progress (outbound only)
- **`AwaitingHandshake`**: TCP connected, waiting for Version and Verack exchange
- **`Connected`**: Handshake complete, normal operation
- **`Rejected`**: Connection rejected due to capacity limits (after handshake)
- **`Failed`**: Connection has failed and should be cleaned up

**State Transitions:**

**Outbound Connections:**
```
Disconnected → Connecting (initiate connection)
Connecting → AwaitingHandshake (TCP established)
Connecting → Disconnected (TCP connection failed - retry with backoff)
AwaitingHandshake → Connected (handshake complete and validated)
AwaitingHandshake → Failed (handshake timeout, error, or validation failed)
Connected → Connecting (network-level failure, restart connection)
Connected → Failed (non-network error or closure)
Failed → Disconnected (cleanup complete)
Disconnected → Connecting (retry attempt after backoff)
```

**Inbound Connections (Normal):**
```
AwaitingHandshake (TCP already established when created)
AwaitingHandshake → Connected (handshake complete and validated)
AwaitingHandshake → Failed (handshake timeout, error, or validation failed)
Connected → AwaitingHandshake (network-level failure, restart connection)
Connected → Failed (non-network error or closure)
Failed → [Actor terminates] (no retry for inbound)
```

**Inbound Connections (Over Capacity):**
```
AwaitingHandshake (TCP already established, reject_after_handshake = true)
AwaitingHandshake → Rejected (handshake complete, send Reject message)
Rejected → [Actor terminates] (connection closed)
```

**Handshake State Details:**

While in `AwaitingHandshake` state, the connection tracks:
- **`version_sent`**: Boolean - we have sent Version message
- **`version_received`**: Boolean - we have received Version message from peer
- **`verack_sent`**: Boolean - we have sent Verack message
- **`verack_received`**: Boolean - we have received Verack message from peer

**Important**: There is no defined order for receiving Version and Verack messages from the peer. The implementation must handle both messages arriving in any order. The handshake is complete when all four flags are true.

**Handshake Differences by Connection Type:**
- **Outbound**: We send Version first, then react to peer's messages
- **Inbound (normal)**: We expect Version from peer first, then send our Version and Verack
- **Inbound (over capacity)**: Same as normal, but after handshake send Reject and disconnect

### 6. Connection Handshake Protocol

The handshake must complete successfully before normal message exchange can occur.

**Handshake Sequence - Outbound Connection:**
1. **Connection established** (TCP connection opened, state → `AwaitingHandshake`)
2. **Send Version message** to peer (`version_sent = true`)
3. **Receive messages from peer** (in any order):
   - When Version received: 
     - **Validate network (mainnet/testnet/regtest) matches our configuration**
     - **Validate blockchain type (must be Bitcoin SV)**
     - **Check user agent against banned list**
     - If validation fails, transition to `Failed` and mark peer as `Banned`
     - If validation succeeds, send Verack in response (`version_received = true`, `verack_sent = true`)
   - When Verack received: `verack_received = true`
4. **Handshake complete** when all flags are true and validation passed:
   - `version_sent && version_received && verack_sent && verack_received`
5. **Transition to Connected state**
6. **Post-handshake**: Send SendHeaders message

**Handshake Sequence - Inbound Connection (Normal):**
1. **Connection accepted** (TCP connection already established, state → `AwaitingHandshake`)
2. **Wait for Version message** from peer
3. **When Version received** from peer:
   - **Validate network (mainnet/testnet/regtest) matches our configuration**
   - **Validate blockchain type (must be Bitcoin SV)**
   - **Check user agent against banned list**
   - If validation fails, transition to `Failed` and mark peer as `Banned`
   - If validation succeeds:
     - `version_received = true`
     - Send our Version message (`version_sent = true`)
     - Send Verack in response to peer's Version (`verack_sent = true`)
4. **Wait for Verack** from peer
5. **When Verack received**: `verack_received = true`
6. **Handshake complete** when all flags are true and validation passed
7. **Transition to Connected state**
8. **Post-handshake**: Send SendHeaders message

**Handshake Sequence - Inbound Connection (Over Capacity):**
1. **Connection accepted** (TCP connection already established, `reject_after_handshake = true`, state → `AwaitingHandshake`)
2. **Wait for Version message** from peer
3. **When Version received** from peer:
   - **Validate network/blockchain/user agent** (same as normal)
   - If validation fails, transition to `Failed` and mark peer as `Banned`
   - If validation succeeds:
     - `version_received = true`
     - Send our Version message (`version_sent = true`)
     - Send Verack in response to peer's Version (`verack_sent = true`)
4. **Wait for Verack** from peer
5. **When Verack received**: `verack_received = true`
6. **Handshake complete** when all flags are true and validation passed
7. **Transition to Rejected state**
8. **Send Reject message** to peer (indicating connection limit reached)
9. **Close connection and terminate actor**

**Validation Rules:**
- **Network Mismatch**: If peer's network (mainnet/testnet/regtest) doesn't match our configured network → Ban peer
- **Blockchain Mismatch**: If peer is not a Bitcoin SV node → Ban peer
- **Banned User Agent**: If peer's user agent string matches a banned pattern → Ban peer

**Assumption**: There is generally only one type of node per IP address. Therefore, banning applies to the IP address, not just a specific peer ID.

**Possible Message Orderings:**
```
Outbound Example:
Us → Peer: Version
Peer → Us: Version (validate, then send Verack if OK)
Us → Peer: Verack
Peer → Us: Verack
[Handshake Complete]

Inbound Example (Normal):
Peer → Us: Version (validate, then send Version and Verack if OK)
Us → Peer: Version
Us → Peer: Verack
Peer → Us: Verack
[Handshake Complete]

Inbound Example (Over Capacity):
Peer → Us: Version (validate, then send Version and Verack if OK)
Us → Peer: Version
Us → Peer: Verack
Peer → Us: Verack
[Handshake Complete]
Us → Peer: Reject
[Connection Closed]
```

**Error Handling:**
- Timeout if handshake does not complete within `handshake_timeout`
- Transition to `Failed` state if handshake fails
- **Update peer status to `Banned` if validation fails**
- Update peer status in peer store accordingly

### 7. Connection Restart After Network Failures

When a connection is in the `Connected` state and experiences a network-level failure, it should be automatically restarted.

**Network-Level Failures (trigger restart):**
- **Connection Reset**: TCP connection reset by peer
- **Connection Lost**: Network connectivity lost
- **Broken Pipe**: Write to closed connection
- **I/O Errors**: Other network I/O errors

**Restart Behavior:**

**For Outbound Connections:**
1. Connection in `Connected` state experiences network failure
2. Log at INFO level: "Connection lost to peer, restarting..."
3. Transition to `Connecting` state (bypass `Disconnected`)
4. **Do not increment retry_count** (this is a restart, not an initial connection failure)
5. Immediately attempt to reconnect (no backoff delay)
6. If reconnection fails, then apply normal retry logic with backoff

**For Inbound Connections:**
1. Connection in `Connected` state experiences network failure
2. Log at INFO level: "Connection lost from peer, attempting restart..."
3. Close existing TCP connection
4. Transition to `AwaitingHandshake` state
5. Attempt to establish new outbound connection to the peer's IP:port
6. If successful, proceed with handshake as outbound connection
7. If unsuccessful, transition to `Failed` and terminate actor (inbound connections don't persist)

**Non-Network Failures (do not restart):**
- Handshake validation failures (ban instead)
- Protocol violations
- Explicit disconnect commands
- Graceful shutdown

**Restart Limits:**
- **`max_restarts`**: Maximum number of automatic restarts before marking as failed (configurable, default: 3)
- **`restart_window`**: Time window for counting restarts (configurable, default: 1 hour)
- If restarts exceed limit within window, mark peer as `Inaccessible` and stop

### 8. Peer Banning

**Ban Triggers:**
- **Network mismatch**: Peer is on a different network (mainnet vs testnet vs regtest)
- **Blockchain mismatch**: Peer is not a Bitcoin SV node
- **Banned user agent**: Peer's user agent matches a banned pattern

**Ban Behavior:**
- When a peer is banned, update peer status to `Banned` in the peer store
- The ban applies to the IP address (since we assume one node type per IP)
- **Outbound**: Do not attempt to connect to banned peers
- **Inbound**: Drop connections from banned IP addresses immediately (before handshake)
- Banned peers are stored in the peer store with status `Banned`

**Ban Configuration:**
- **`banned_user_agents`**: List of user agent string patterns to ban (configurable, supports wildcards or regex)

**Ban Management:**
- Bans persist across restarts (stored in peer store file)
- External users can manually ban/unban peers through Manager interface
- Bans can be temporary (with expiry timestamp) or permanent (future enhancement)

### 9. Reconnection Backoff Strategy

When a connection fails or becomes inaccessible, an exponential backoff strategy controls retry attempts.

**Applies to**: Outbound connections only. Inbound connections do not retry.

**Backoff Parameters (all configurable):**
- **`initial_backoff`**: Starting backoff delay (default: 5 seconds)
- **`max_retries`**: Maximum number of retry attempts (default: 10)
- **`backoff_multiplier`**: Multiplier for exponential backoff (default: 2.0)

**Backoff Algorithm:**
```
delay = initial_backoff * (backoff_multiplier ^ retry_count)
```

**Retry Triggering Conditions:**
The following TCP-level errors during connection establishment should trigger retry with backoff:
- **Connection Refused**: Peer is not listening on the port
- **Connection Timeout**: Peer did not respond within timeout
- **Connection Reset**: Connection was reset during establishment

**Non-Retry Conditions:**
The following conditions should NOT trigger retry (transition directly to Failed):
- **Handshake validation failures** (network/blockchain/user agent mismatch) → Ban peer
- **Handshake timeout** after TCP connection established → Mark Inaccessible

**Retry Behavior:**
1. First retry after `initial_backoff` seconds (5s default)
2. Second retry after 10s
3. Third retry after 20s
4. Fourth retry after 40s
5. And so on...
6. After `max_retries` attempts, mark peer as `Inaccessible` and stop retrying
7. Reset retry count when connection succeeds

**Status Updates:**
- During retries: peer status remains `Unknown` or previous status
- After max retries exceeded: peer status → `Inaccessible`
- On successful connection: peer status → `Valid`, retry count reset
- On validation failure during handshake: peer status → `Banned`, no further retries

### 10. Keepalive Protocol (Ping/Pong)

A periodic keepalive mechanism ensures the connection remains active and responsive.

**Ping Protocol:**
- **Outbound Pings**: Send Ping message periodically (default: every 5 minutes, configurable)
- **Ping Response**: Expect Pong message in response to our Ping within a timeout period
- **Inbound Pings**: When Ping message is received from peer, send Pong message in response

**Configuration:**
- `ping_interval`: Time between outbound Ping messages (default: 5 minutes)
- `ping_timeout`: Maximum time to wait for Pong response (configurable)

**Failure Handling:**
- If Pong not received within timeout, connection may be considered stale
- Transition to `Failed` state and close connection
- Peer status updated accordingly (not banned, just disconnected)

### 11. SendHeaders Protocol

After handshake completion, the SendHeaders message negotiates how block announcements are communicated.

**SendHeaders Flow:**
- **After handshake**: PeerConnection sends SendHeaders message to peer
- **If SendHeaders received** from peer:
  - Set `send_headers_mode = true` flag for this connection
  - Future block announcements should send full headers instead of inventory messages

**Connection Flags:**
- `send_headers_mode`: Boolean indicating peer preference for headers vs inventory

### 12. Bitcoin Protocol Messages

**Message Handling:**
All Bitcoin protocol messages use the existing **`Message` enum from the p2p module**.

**Message Types Used:**
- `Message::Version` - Handshake version information (contains network, blockchain type, user agent)
- `Message::Verack` - Handshake acknowledgment
- `Message::Ping` - Keepalive ping
- `Message::Pong` - Keepalive pong response
- `Message::SendHeaders` - Request headers instead of inventory
- **`Message::Reject`** - Rejection message (sent when connection limit exceeded)
- Other message types as defined in the existing p2p module

**Version Message Contents (for validation):**
- Network identifier (mainnet/testnet/regtest)
- Blockchain type identifier (must be Bitcoin SV)
- User agent string (checked against banned list)

**Reject Message Usage:**
- Sent to peers when connection count is at or above `max_connections`
- Sent after successful handshake (to be protocol-compliant)
- Connection is closed immediately after sending Reject

### 13. Logging and Observability

The module provides comprehensive logging and observability support.

**Logging Framework:**
- **Primary**: Use `tracing` crate (standard Rust structured logging)
- **Compatibility**: Also support `log` facade for compatibility

**Log Levels:**
- **ERROR**: Critical failures, listener bind failures, configuration errors
- **WARN**: Connection failures, individual accept failures, retry attempts
- **INFO**: Connection established/lost, handshake complete, configuration updates, restarts
- **DEBUG**: Detailed protocol messages, state transitions, peer discovery
- **TRACE**: Very detailed logging for debugging (message contents, etc.)

**Configurable Logging:**
- **`log_level`**: Configurable minimum log level (default: INFO)
- Can be configured per-component (Manager, PeerConnection, PeerStore, etc.)
- Runtime log level adjustment via configuration updates

**OpenTelemetry Support:**
- **Traces**: Span for each connection lifecycle, handshake, message exchange
- **Metrics**:
  - Connection count (gauge)
  - Connections by status (Valid/Inaccessible/Banned) (gauge)
  - Connection attempts (counter)
  - Connection failures by reason (counter)
  - Handshake duration (histogram)
  - Messages sent/received by type (counter)
  - Ban events (counter)
  - Restart events (counter)
- **Events**: Key lifecycle events (connection established, banned, failed, etc.)
- **Resource Attributes**: Network type, node version, etc.

**OpenTelemetry Configuration:**
- **`enable_telemetry`**: Boolean to enable/disable OpenTelemetry (default: false)
- **`telemetry_endpoint`**: OTLP endpoint URL (configurable)
- **`service_name`**: Service name for telemetry (default: "bsv-p2p-manager")

**Structured Logging Fields:**
- `peer_id`: UUID of peer
- `peer_address`: IP:port of peer
- `connection_type`: "inbound" or "outbound"
- `state`: Current connection state
- `error`: Error message (when applicable)
- `network`: Bitcoin network (mainnet/testnet/regtest)

### 14. Peer Data Structure

Each peer in the known peers list has the following attributes:

- **`id`**: UUID - Unique identifier for the peer
- **`ip_address`**: IP address - Can be IPv4 or IPv6
- **`port`**: Port number - TCP port for connection
- **`status`**: Peer status - One of:
  - `Valid`: Peer is known to be accessible and working
  - `Inaccessible`: Peer cannot currently be reached (after max retries exceeded)
  - `Banned`: Peer is banned and should not be contacted (network/blockchain/user agent mismatch)
  - `Unknown`: Peer status has not yet been determined
- **`status_timestamp`**: Timestamp - Records when the status was last determined/updated

### 15. PeerStore Trait

A trait that defines the interface for storing and managing peer data. This allows for different storage backend implementations.

**Required Methods:**

**Basic CRUD:**
- **`create(peer: Peer)`**: Add a new peer to the store
- **`read(id: Uuid)`**: Retrieve a peer by ID
- **`update(peer: Peer)`**: Modify an existing peer's information
- **`delete(id: Uuid)`**: Remove a peer from the store
- **`list_all()`**: Retrieve a list of all peers in the store

**Query Methods:**
- **`find_by_status(status: PeerStatus)`**: Find all peers with a specific status
- **`find_by_ip_port(ip: IpAddr, port: u16)`**: Find a peer by IP address and port (for duplicate detection and ban checking)
- **`count_by_status(status: PeerStatus)`**: Count peers with a specific status

**Design Principles:**
- Trait-based design allows for custom implementations
- All methods should be async to support non-blocking I/O operations
- Proper error handling for storage operations
- Methods return `Result<T, Error>` for error handling

### 16. Built-in PeerStore Implementation

**InMemoryPeerStore**: Default implementation of the `PeerStore` trait.

**Characteristics:**
- Stores peer data in memory during runtime (using `HashMap<Uuid, Peer>`)
- Secondary index by IP:port for duplicate detection and ban checking
- **Persistence on shutdown**: Serializes peer data to a file when the Manager shuts down
- **Loading on startup**: Deserializes peer data from file when the Manager starts
- Fast in-memory operations during runtime
- File-based persistence for durability across restarts

**File Format Considerations:**
- Should use a standard serialization format (JSON, CBOR, or bincode)
- File path should be configurable
- Handle missing or corrupted files gracefully on startup

### 17. DNS Peer Discovery

The Manager implements an automated DNS-based peer discovery mechanism.

**Discovery Schedule:**
- **At startup**: Execute immediately when Manager starts (in normal mode only)
- **Periodic execution**: Once every hour during runtime (in normal mode only)

**Discovery Process:**
1. Query each DNS name in the configured DNS seeds list
2. Each DNS query may return multiple IP addresses
3. For each returned IP address:
   - Check for duplicates using `find_by_ip_port()`
   - **Skip if IP:port is already in store with status `Banned`**
   - If not a duplicate, create a new peer entry with:
     - Generated UUID as `id`
     - Returned IP address (IPv4 or IPv6)
     - Default port from configuration
     - Status set to `Unknown`
     - `status_timestamp` set to current time
   - Add to the peer store

**Configuration Requirements:**
The DNS discovery process requires these configuration elements:
- **`dns_seeds`**: Set/list of DNS names for peer discovery
- **`default_port`**: Default port number to use for discovered peers

### 18. Manager Operating Modes

The Manager supports two distinct operating modes:

#### Normal Mode (Default)
**Characteristics:**
- Connects to peers from the peer store (outbound)
- Accepts inbound connections (if listener enabled)
- Uses DNS peer discovery for finding new peers
- **Maintains connection count between `target_connections` and `max_connections`**
- Prioritizes outbound connection attempts in the following order:
  1. First, connect to peers with status `Valid`
  2. Then, connect to peers with status `Unknown`
  3. **Skip peers with status `Banned` or `Inaccessible`**
  4. **Continue until total connections reach `target_connections`**

**Connection Strategy:**
- **While `total_connections < target_connections`**: Actively initiate new outbound connections
- **When `target_connections <= total_connections < max_connections`**: Accept inbound but don't initiate outbound
- **When `total_connections >= max_connections`**: Reject new inbound connections (after handshake with Reject message)
- DNS discovery runs on startup and hourly
- Implements backoff strategy for failed outbound connections
- Inbound connections count toward total connection count
- **Rejects inbound connections from banned IP addresses**

#### Fixed Peer List Mode
**Characteristics:**
- Started with a defined list of specific peers (passed as startup argument)
- **Only** initiates outbound connections to peers in the provided list (excluding banned peers)
- **Still accepts** inbound connections (if listener enabled), subject to `max_connections` limit
- **Ignores** the `target_connections` configuration for outbound connection decisions
- **Does not** initiate connections to newly discovered peers (from DNS or other sources)
- DNS discovery may still run but discovered peers are not used for outbound connections

**Connection Strategy:**
- Initiates outbound connections only to the explicitly specified peers (if not banned)
- Inbound connections are still accepted and managed normally (excluding banned IPs, respecting max_connections)
- Number of outbound connections determined solely by the provided peer list
- Provides deterministic, controlled peer connectivity
- Still implements backoff strategy for failed outbound connections to fixed peers
- **Fixed peers can still be banned if validation fails**
- **Still respects `max_connections` limit for total connections**

**Use Cases:**
- Testing with specific known peers
- Private/permissioned network setups
- Debugging specific peer interactions
- Connecting only to trusted nodes

### 19. Configuration Object
A complex, extensible configuration structure that controls both Manager and PeerConnection behavior.

**Configuration Scope:**
- **Manager-level settings**: Apply to the Manager's operation
- **Connection-level settings**: Apply to individual PeerConnection instances

**Configuration Elements:**

**Manager-Level:**
- `network`: Specifies the Bitcoin SV network type
  - `mainnet`
  - `testnet`
  - `regtest`
- **`target_connections`**: Target number of total connections to maintain (default: 8)
- **`max_connections`**: Maximum number of total connections allowed (default: 20)
- `dns_seeds`: Set/list of DNS names for peer discovery
- `default_port`: Default port for discovered peers
- `peer_store_file_path`: File path for persisting peer data (optional, for InMemoryPeerStore)
- `enable_listener`: Boolean - whether to enable inbound connection listener (default: false)
- `listener_address`: IP address to bind listener to (e.g., "0.0.0.0" or "::0", default: "0.0.0.0")
- `listener_port`: Port to bind listener to (default: same as `default_port`)
- `banned_user_agents`: List of user agent string patterns to ban (supports wildcards or regex)
- **`log_level`**: Minimum log level (ERROR, WARN, INFO, DEBUG, TRACE) (default: INFO)
- **`enable_telemetry`**: Enable OpenTelemetry (default: false)
- **`telemetry_endpoint`**: OTLP endpoint URL (optional)
- **`service_name`**: Service name for telemetry (default: "bsv-p2p-manager")

**Connection-Level:**
- `ping_interval`: Duration between outbound Ping messages (default: 5 minutes)
- `ping_timeout`: Maximum time to wait for Pong response
- `handshake_timeout`: Maximum time to wait for handshake completion
- `initial_backoff`: Starting backoff delay for reconnection attempts (default: 5 seconds)
- `max_retries`: Maximum number of connection retry attempts (default: 10)
- `backoff_multiplier`: Multiplier for exponential backoff (default: 2.0)
- **`max_restarts`**: Maximum automatic restarts for network failures (default: 3)
- **`restart_window`**: Time window for counting restarts (default: 1 hour)

**Design Principles:**
- Must be extensible for future configuration options
- Clear separation between manager-level and connection-level settings
- Supports dynamic updates at runtime
- All timeouts and retry parameters are configurable
- **`target_connections` must be less than or equal to `max_connections`**

### 20. Dynamic Configuration Update Mechanism

**Update Flow:**
1. External user submits updated configuration to the Manager
2. Manager processes the configuration update
3. Manager applies relevant changes to its own behavior
4. Manager propagates connection-level configuration changes to all active `PeerConnection` instances
5. Manager applies any necessary changes to channels (if applicable)
6. Each `PeerConnection` applies the updated configuration to its operation

**Update Considerations:**
- Configuration updates should be validated before application
- Some configuration changes may require connection restarts or adjustments
- The system should handle partial updates gracefully
- Configuration updates should be atomic where possible
- Updates to `dns_seeds` or `default_port` affect future discovery runs
- Updates to `ping_interval`, timeouts, or backoff parameters apply to active connections
- Listener configuration changes (`enable_listener`, `listener_address`, `listener_port`) may require stopping and restarting the listener
- **Updates to `banned_user_agents` apply to future handshakes; existing connections are not re-validated**
- **Updates to `target_connections` or `max_connections` affect future connection decisions**
- **If `max_connections` is reduced below current connection count, existing connections are not forcibly closed**
- **Updates to `log_level` take effect immediately**

## Internal Message Types

Messages passed through Tokio channels between components.

### Manager → PeerConnection Command Messages
```rust
enum PeerConnectionCommand {
    UpdateConfig(ConnectionConfig),
    Disconnect,
    SendMessage(Message),  // Message from p2p module
}
```

### PeerConnection → Manager Event Messages (via Broadcast)

**Control Events:**
```rust
enum ControlEvent {
    ConnectionEstablished { peer_id: Uuid, is_inbound: bool },
    ConnectionFailed { peer_id: Uuid, reason: String },
    ConnectionLost { peer_id: Uuid },
    ConnectionRestarting { peer_id: Uuid, reason: String },
    HandshakeComplete { peer_id: Uuid },
    PeerMisbehavior { peer_id: Uuid, reason: String },
    PeerBanned { peer_id: Uuid, ip: IpAddr, reason: BanReason },
    InboundConnectionAccepted { peer_id: Uuid, address: SocketAddr },
    InboundConnectionRejected { address: SocketAddr, reason: String },
    ConnectionRejectedCapacity { peer_id: Uuid, address: SocketAddr },
    ListenerBindFailed { address: SocketAddr, error: String },
}

enum BanReason {
    NetworkMismatch { expected: Network, received: Network },
    BlockchainMismatch { received: String },
    BannedUserAgent { user_agent: String },
}
```

**Bitcoin Message Events:**
```rust
struct BitcoinMessageEvent {
    peer_id: Uuid,
    message: Message,  // Message from p2p module
}
```

## Error Types

```rust
enum PeerManagerError {
    // Connection errors (retryable)
    ConnectionRefused,
    ConnectionTimeout,
    ConnectionReset,
    
    // Connection errors (non-retryable)
    ConnectionFailed(String),
    
    // Handshake errors (non-retryable, lead to ban)
    HandshakeTimeout,
    HandshakeFailed(String),
    NetworkMismatch { expected: Network, received: Network },
    BlockchainMismatch { received: String },
    BannedUserAgent { user_agent: String },
    
    // Store errors
    PeerStoreError(String),
    PeerNotFound(Uuid),
    DuplicatePeer,
    
    // Configuration errors
    InvalidConfiguration(String),
    InvalidConnectionLimits { target: usize, max: usize },
    
    // DNS errors
    DnsResolutionFailed(String),
    
    // Listener errors (non-fatal)
    ListenerBindFailed(String),
    ListenerAcceptFailed(String),
    
    // Channel errors
    ChannelSendError,
    ChannelReceiveError,
    
    // I/O errors
    IoError(std::io::Error),
}
```

## Error Handling Strategy

### Connection Establishment Errors

**Retryable Errors** (trigger backoff retry):
- `ConnectionRefused`: TCP connection refused by peer
- `ConnectionTimeout`: TCP connection timed out
- `ConnectionReset`: TCP connection reset during establishment
- Action: Retry with exponential backoff up to `max_retries`, then mark as `Inaccessible`

**Non-Retryable Errors** (immediate failure):
- `HandshakeTimeout`: Handshake did not complete in time after TCP established
- Action: Mark peer as `Inaccessible`, do not retry immediately

**Ban-Triggering Errors** (immediate ban, no retry):
- `NetworkMismatch`: Peer is on wrong network
- `BlockchainMismatch`: Peer is not Bitcoin SV
- `BannedUserAgent`: Peer's user agent is banned
- Action: Mark peer as `Banned`, never retry, reject future inbound connections

### Operational Errors

**During Connected State (trigger restart):**
- Network-level failures (connection reset, broken pipe, I/O errors): Restart connection
- Restart up to `max_restarts` times within `restart_window`
- After max restarts exceeded: Mark as `Inaccessible`

**During Connected State (no restart):**
- Ping timeout: Close connection, mark as `Inaccessible`, allow future retry
- Protocol violation: May trigger ban depending on severity
- Explicit disconnect: Clean shutdown, no restart

**Store Errors:**
- Should be logged and potentially exposed to user
- May indicate corruption or disk issues
- Should not crash the Manager

**Listener Errors (Non-Fatal):**
- Bind failures: Log at ERROR level, continue with outbound connections only
- Individual accept failures: Log at WARN level and continue accepting
- Manager continues to operate without listener

**Configuration Errors:**
- `InvalidConnectionLimits`: Reject configuration where `target_connections > max_connections`

## Communication Architecture

### Control Flow
- Manager → PeerConnection: Commands and configuration updates sent via Tokio channels
- PeerConnection → Manager: Events broadcast via shared broadcast channels
- External Consumers → Manager: Subscribe to broadcast channels for event notifications
- External Consumers → Manager: Submit configuration updates
- Manager → DNS Servers: Periodic queries for peer discovery (Normal Mode only)
- Manager ↔ PeerStore: CRUD and query operations for peer management
- Inbound Peers → Listener: TCP connection attempts
- Listener → PeerStore: Check if IP is banned before accepting
- **Listener → Manager: Check connection count before accepting**
- Listener → Manager: Notification of new inbound connection
- Manager → PeerConnection: Create new actor for inbound connection (with reject_after_handshake flag if over capacity)
- PeerConnection → PeerStore: Update peer status (Valid/Inaccessible/Banned)
- PeerConnection ↔ Peer: Bitcoin protocol message exchange via single bidirectional TCP channel (using Message enum from p2p module)

### Message Types
1. **Control Messages**: Administrative and connection management messages (ControlEvent)
2. **Bitcoin Messages**: Bitcoin SV protocol messages (BitcoinMessageEvent wrapping Message enum)
3. **Configuration Update Messages**: Dynamic configuration change commands (PeerConnectionCommand)
4. **Handshake Messages**: Message::Version, Message::Verack
5. **Keepalive Messages**: Message::Ping, Message::Pong
6. **Protocol Negotiation Messages**: Message::SendHeaders
7. **Rejection Messages**: Message::Reject (sent when over capacity)

## Design Patterns
- **Actor Model**: Each Manager and PeerConnection operates as an independent actor
- **Message Passing**: All communication via Tokio channels
- **Broadcast/Subscribe**: Event distribution using Tokio broadcast channels
- **Dynamic Reconfiguration**: Runtime configuration updates without system restart
- **Periodic Tasks**: Automated DNS discovery and ping keepalive on fixed schedules
- **Strategy Pattern**: PeerStore trait allows pluggable storage implementations
- **Repository Pattern**: PeerStore abstracts peer data persistence
- **State Machine**: Connection lifecycle follows an explicit state machine with defined transitions
- **Exponential Backoff**: Reconnection attempts use exponential backoff with configurable parameters
- **Listener Pattern**: Inbound connection listener accepts and delegates connections
- **Validation Pattern**: Handshake validation with ban enforcement
- **Capacity Management**: Connection limits with graceful rejection
- **Resilience Pattern**: Automatic restart on network failures
- **Observability Pattern**: Comprehensive logging and telemetry

## Lifecycle Management

### Startup Sequence - Normal Mode
1. Manager initializes with configuration
2. **Validate configuration (ensure `target_connections <= max_connections`)**
3. **Initialize logging and OpenTelemetry (if enabled)**
4. PeerStore loads persisted peer data from file (if exists)
5. **Attempt to start inbound connection listener on configured address/port**
6. **If listener bind fails:**
   - Log error at ERROR level
   - Emit telemetry event
   - Continue startup (non-fatal)
7. **If listener bind succeeds:**
   - Log success at INFO level
   - Begin accepting connections
8. DNS Peer Discovery executes (skips banned IPs)
9. Manager reads all peers from peer store using `find_by_status()`
10. **Manager tracks total connection count**
11. Manager initiates outbound connections in priority order:
   - First to peers with status `Valid`
   - Then to peers with status `Unknown`
   - **Skip peers with status `Banned` or `Inaccessible`**
   - **Until total connections reach `target_connections`**
12. Each PeerConnection executes handshake protocol with validation
13. Listener continuously accepts new inbound connections:
    - Rejecting banned IPs
    - **Creating PeerConnections with reject_after_handshake=true if total_connections >= max_connections**
14. **Manager monitors connection count and initiates new outbound connections if count drops below target**
15. Manager continues normal operation with periodic DNS discovery

### Startup Sequence - Fixed Peer List Mode
1. Manager initializes with configuration and fixed peer list argument
2. **Validate configuration (ensure `target_connections <= max_connections`)**
3. **Initialize logging and OpenTelemetry (if enabled)**
4. PeerStore loads persisted peer data from file (if exists)
5. **Attempt to start inbound connection listener on configured address/port**
6. **If listener bind fails:**
   - Log error at ERROR level
   - Emit telemetry event
   - Continue startup (non-fatal)
7. **If listener bind succeeds:**
   - Log success at INFO level
   - Begin accepting connections
8. DNS Peer Discovery may execute but results are not used for outbound connections
9. Manager initiates outbound connections **only** to peers in the provided fixed list (excluding banned peers)
10. Each PeerConnection executes handshake protocol with validation
11. Listener continuously accepts new inbound connections:
    - Rejecting banned IPs
    - **Creating PeerConnections with reject_after_handshake=true if total_connections >= max_connections**
12. Manager continues normal operation without attempting additional outbound connections

### Connection Lifecycle - Outbound
1. State: `Disconnected`
2. State: `Connecting` - TCP connection attempt initiated
3. **If connection refused/timeout/reset**: Return to `Disconnected`, retry with backoff
4. State: `AwaitingHandshake` - TCP established, send Version
5. **Receive and validate peer's Version message**
6. **If validation fails (network/blockchain/user agent)**: Transition to `Failed`, mark peer as `Banned`, no retry
7. **If validation succeeds**: Complete handshake exchange
8. State: `Connected` - Handshake complete, send SendHeaders
9. **Manager increments connection count**
10. Periodic Ping/Pong keepalive begins
11. Normal message exchange
12. **If network-level failure**: Log restart, transition to `Connecting`, attempt immediate reconnect (no backoff)
13. **If non-network failure**: Transition to `Failed`
14. State: `Failed` or network restart
15. State: `Disconnected` - Cleanup complete (if Failed)
16. **Manager decrements connection count**
17. Retry with backoff strategy (if not banned and not exceeded max_retries)

### Connection Lifecycle - Inbound (Normal)
1. **Listener accepts TCP connection**
2. **Listener checks if IP is banned, drops if banned**
3. **Manager checks connection count: `total_connections < max_connections`**
4. **Manager extracts peer info and checks for duplicate**
5. **Manager creates new PeerConnection actor with established TCP connection**
6. State: `AwaitingHandshake` - Wait for Version from peer
7. **Receive and validate peer's Version message**
8. **If validation fails (network/blockchain/user agent)**: Transition to `Failed`, mark peer as `Banned`, terminate actor
9. **If validation succeeds**: Send Version and Verack
10. Handshake exchange completes
11. State: `Connected` - Handshake complete, send SendHeaders
12. **Manager increments connection count**
13. Periodic Ping/Pong keepalive begins
14. Normal message exchange
15. **If network-level failure**: Log restart, close connection, attempt outbound reconnect
16. **If non-network failure**: Transition to `Failed`
17. State: `Failed` or reconnect attempt
18. **Manager decrements connection count**
19. Actor terminates (or continues as outbound if reconnected)

### Connection Lifecycle - Inbound (Over Capacity)
1. **Listener accepts TCP connection**
2. **Listener checks if IP is banned, drops if banned**
3. **Manager checks connection count: `total_connections >= max_connections`**
4. **Manager creates new PeerConnection actor with `reject_after_handshake = true`**
5. State: `AwaitingHandshake` - Wait for Version from peer
6. **Receive and validate peer's Version message**
7. **If validation fails**: Transition to `Failed`, mark peer as `Banned`, terminate actor
8. **If validation succeeds**: Send Version and Verack
9. Handshake exchange completes
10. State: `Rejected`
11. **Send Reject message to peer (indicating capacity limit)**
12. **Close connection**
13. **Broadcast ConnectionRejectedCapacity event**
14. Actor terminates (connection never counted toward total)

### Shutdown Sequence
1. Manager receives shutdown signal
2. **Log shutdown at INFO level**
3. Stop accepting new inbound connections (close listener)
4. Manager sends Disconnect command to all PeerConnections
5. Active connections are gracefully closed
6. PeerStore persists current peer data to file
7. **Flush telemetry data**
8. Manager terminates

## Status Timestamp Management

**When status_timestamp is updated:**
- When a peer is first discovered (set to discovery time)
- When a connection to a peer succeeds (status → Valid, after successful handshake and validation, retry count reset)
- When max retries exceeded (status → Inaccessible) - outbound only
- **When a peer is banned (status → Banned, validation failed)**
- Any time the peer status changes
- When an inbound connection is accepted from a new peer

**Use cases for status_timestamp:**
- Determine when to retry connecting to Inaccessible peers
- Track how long a peer has been Banned
- Identify stale peer data that may need re-validation
- Support time-based peer selection strategies
- Calculate backoff delays

## Future Enhancements

### Dual-Channel Support (Future)
The architecture is designed to support up to 2 TCP channels per peer connection in a future update. The second channel will provide:
- Increased bandwidth for high-throughput scenarios
- Redundancy and failover capabilities
- Load balancing across channels

The initial single-channel implementation with bidirectional communication provides a foundation that can be extended without major architectural changes.

### Ban Enhancements (Future)
- **Temporary bans**: Bans with expiry timestamps
- **Ban severity levels**: Different ban durations based on violation type
- **Whitelist**: Ability to whitelist certain IPs that should never be banned
- **Ban appeal mechanism**: Way to unban peers programmatically

### Advanced Connection Management (Future)
- **Connection quality metrics**: Track latency, reliability, throughput per peer
- **Intelligent peer selection**: Prefer peers with better metrics
- **Geographic diversity**: Ensure connections span multiple regions
- **Automatic rebalancing**: Periodically replace low-quality connections

---

*This is a living document. Additional design details and configuration options will be added as the design evolves.*

