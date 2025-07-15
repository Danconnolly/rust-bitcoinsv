# rust-bitcoinsv

![Test Status](https://gist.githubusercontent.com/Danconnolly/202d737d8ec36a48fbb4f7d0d4e1d779/raw/badge.svg)
[![dependency status](https://deps.rs/repo/github/Danconnolly/rust-bitcoinsv/status.svg)](https://deps.rs/repo/github/Danconnolly/rust-bitcoinsv)

NOTE: This library is incomplete and undergoing extensive changes at the moment.

## Breaking Changes in v0.4.0

The `PrivateKey::from<String>` implementation has been changed to use `TryFrom<String>` to improve error handling. Update your code:

```rust
// Before (v0.3.x)
let key = PrivateKey::from(wif_string);

// After (v0.4.0)
let key = PrivateKey::try_from(wif_string)?;
```

This library is a start at building a high-performance Bitcoin SV library in Rust.

This is a hobby project and the code is experimental. If you're looking for a complete library, check
out [rust-sv](https://docs.rs/sv/latest/sv/)
by Brenton Gunning. Progress on this library is determined by the needs of various other projects.

If you have anything you particularly want to see, feel free to open an issue or start a discussion.

## Current Feature Status

* `bitcoin` module: main structs - Tx, BlockHeader, Block
* `util` module: main structs - Amount




