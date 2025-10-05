# P2P Integration Tests

This directory contains integration tests for the P2P module that test against real Bitcoin SV nodes.

## Running Integration Tests

By default, integration tests will be skipped if no test node is configured. To run them against a real BSV node:

```bash
# Set the test node address
export BSV_TEST_NODE=your.bsv.node:8333

# Run all integration tests
cargo test --test p2p_integration_tests

# Run a specific test
cargo test --test p2p_integration_tests test_tcp_connection_to_remote_node -- --nocapture
```

## Configuration

The following environment variables control integration test behavior:

- **BSV_TEST_NODE**: Address of the BSV node to test against (format: `IP:PORT` or `hostname:PORT`)
  - Example: `BSV_TEST_NODE=seed.bitcoinsv.io:8333`
  - If not set, integration tests will be skipped with a warning

- **BSV_TEST_NETWORK**: Network type (`mainnet`, `testnet`, or `regtest`)
  - Default: `mainnet`

- **BSV_TEST_TIMEOUT**: Timeout in seconds for connection attempts
  - Default: `30`

## Example Usage

```bash
# Test against a local regtest node
BSV_TEST_NODE=127.0.0.1:18444 BSV_TEST_NETWORK=regtest cargo test --test p2p_integration_tests

# Test with increased timeout
BSV_TEST_NODE=seed.bitcoinsv.io:8333 BSV_TEST_TIMEOUT=60 cargo test --test p2p_integration_tests

# Run with verbose logging
RUST_LOG=debug BSV_TEST_NODE=seed.bitcoinsv.io:8333 cargo test --test p2p_integration_tests -- --nocapture
```

## Test Coverage

Current integration tests:

1. **test_tcp_connection_to_remote_node**: Verifies basic TCP connectivity to a BSV node
2. **test_send_version_and_receive_header**: Tests sending Version message and receiving response
3. **test_complete_handshake_with_remote_node**: Tests full handshake protocol (Version + Verack exchange)

## Notes

- These tests require network access to a Bitcoin SV node
- Tests will timeout if the node is not responsive
- For CI/CD, consider running a local BSV node in regtest mode
- Tests are designed to be non-destructive and read-only
