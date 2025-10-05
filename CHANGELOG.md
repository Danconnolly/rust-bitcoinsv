# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.1] - 2025-10-05

### Changed
- Updated `secp256k1` dependency from 0.30.0 to 0.31.1
  - Replaced deprecated `SecretKey::from_slice()` with `from_byte_array()`
  - Replaced deprecated `Message::from_digest_slice()` with `from_digest()`
  - Updated API calls for `sign_ecdsa()` and `verify_ecdsa()` to pass message by value
- Updated `rand` dependency from 0.8.5 to 0.9.2 (required by secp256k1 0.31)
  - Replaced deprecated `rand::thread_rng()` with `rand::rng()`
- Updated various other dependencies to latest compatible versions:
  - proptest: 1.7.0 → 1.8.0
  - serde: 1.0.219 → 1.0.228
  - rustix: 1.0.8 → 1.1.2
  - And 28 other minor dependency updates
- Removed `Cargo.lock` from version control (standard practice for library crates)

## [0.4.0] - 2025-01-15

### Breaking Changes
- Changed `PrivateKey::from<String>` to use `TryFrom<String>` trait instead of `From<String>`. This change prevents panics on invalid WIF strings and returns a proper `Result` instead. Users must now handle the potential error case when converting from strings.

### Security
- **[CRITICAL]** Fixed memory exhaustion vulnerability in transaction parsing that could lead to DoS attacks. Added validation for transaction input/output counts with maximum limits of 100,000 each. Transactions exceeding these limits now return a `BadData` error.

### Added
- Comprehensive test suite with 108+ new tests across multiple categories:
  - Script execution engine tests (26 tests)
  - Signature verification tests (12 tests)
  - Merkle tree operation tests (13 tests)
  - Network protocol (P2P) tests (21 tests)
  - VarInt edge case tests (11 tests)
  - Property-based tests using proptest (12 tests)
  - Stress tests for large data handling (8 tests)
- Fuzzing infrastructure with cargo-fuzz and 5 fuzz targets:
  - Script execution fuzzing
  - VarInt encoding/decoding fuzzing
  - Merkle tree operation fuzzing
  - P2P message parsing fuzzing
  - Transaction parsing fuzzing
- `Display` trait implementation for `BlockHeader`
- `Display` trait implementation for `BlockchainId`
- `Debug` trait implementation for `BlockHeader`
- Exposed `p2p` module in the public API
- `from_slice()` method for `Hash` type
- Additional `From` trait implementations for various type conversions
- Script interpreter with full Bitcoin script operation support
- Signature verification with all SigHash types
- Transaction context support for script evaluation
- Merkle tree building and verification functions
- P2P message framing and protocol handling
- Claude Code integration with CLAUDE.md documentation

### Fixed
- Replaced all `unwrap()` calls with proper error handling throughout the codebase
- Fixed potential panics in various parsing and conversion functions
- Fixed hash hex roundtrip test to use correct encoding method
- Fixed script builder operations test to handle parsing errors gracefully

### Changed
- Improved error handling across the entire library
- Enhanced documentation for various modules and functions

## [0.3.3] - 2025-04-25

Previous release - see git history for details.