# rust-bitcoinsv
![Test Status](https://gist.githubusercontent.com/Danconnolly/202d737d8ec36a48fbb4f7d0d4e1d779/raw/badge.svg)

This library is a start at building a high-performance Bitcoin SV library in Rust. The focus is on using async
paradigms and efficient data structures that minimize memory allocations and copying. The goal is to be able to
handle a blockchain with increasing throughput.

This is a hobby project and the code is experimental. If you're looking for a complete library, check out [rust-sv](https://docs.rs/sv/latest/sv/)
by Brenton Gunning. Progress on this library is determined by the needs of various other projects.

If you have anything you particularly want to see, feel free to open an issue or start a discussion.

## Current Feature Status

* `bitcoin` module: main structs - Tx, BlockHeader, FullBlockStream
* `p2p` module: needs review
* `util` module: main structs - Amount




