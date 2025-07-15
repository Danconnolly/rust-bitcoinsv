# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy

# Build the project
cargo build --verbose

# Run all tests
cargo test

# Run tests with better output (if nextest is installed)
cargo nextest run --profile ci --no-fail-fast

# Run a single test
cargo test test_name

# Security audit
cargo audit
```

## Project Architecture

This is a Rust workspace with a single member library `bsv` that implements Bitcoin SV functionality.

### Core Design Philosophy
- Uses `BlockchainId` instead of network identifiers - blockchains are distinguished by their genesis block
- Focuses on infrastructure components, not wallet or client functionality
- No support for obsolete Bitcoin versions to keep the codebase clean
- Performance-focused with efficient byte handling

### Key Modules and Structures

**bitcoin module** (`bsv/src/bitcoin/`):
- `Tx`: Transaction structure with inputs/outputs
- `Block` and `BlockHeader`: Block data structures
- `Script`: Bitcoin script functionality with builder pattern
- `Hash`: SHA256d hash implementation (Bitcoin's double-SHA256)
- `Address`: Bitcoin address handling
- `PrivateKey`/`PublicKey`: Secp256k1 cryptographic keys

**util module** (`bsv/src/util/`):
- `Amount`: Bitcoin amount handling with satoshi precision

### Encoding Pattern
The codebase uses a custom `Encodable` trait for Bitcoin's binary wire format, separate from serde JSON serialization. Many structures store the raw encoded form and decode on demand for performance.

### Error Handling
Centralized error handling through `bsv/src/result.rs` with a comprehensive `Error` enum and standard `Result<T>` type alias.

### Testing
- Unit tests are embedded in source files using `#[cfg(test)]` modules
- Binary test data is stored in `testdata/` directory
- Use `cargo test specific_test_name` to run individual tests

### Important Implementation Notes
- The P2P protocol implementation ignores checksums for streaming efficiency (see `docs/dev.md`)
- Many data structures are immutable and store raw encoded forms
- Type aliases like `TxHash = Hash` are used throughout for clarity
- The codebase is experimental/hobby project status as noted in README.md

## Development Workflow

- Always create a new branch when you start to work on a different issue

## Repository Management
- Issues are registered on github