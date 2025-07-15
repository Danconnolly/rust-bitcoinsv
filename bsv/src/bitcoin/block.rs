use crate::bitcoin::{varint_decode, varint_size, BlockHeader, BlockchainId, Encodable, Tx};
use crate::{Error, Result};
use bytes::Bytes;
use hex::{FromHex, ToHex};
use std::convert::TryFrom;

/// Contains a full block from the blockchain.
///
/// This can get large, a 4GB block will consume at least 4GB of RAM.
#[derive(Clone, Debug)]
pub struct Block {
    /// The encoded block.
    pub raw: Bytes,
    /// The number of transactions in the block.
    pub num_tx: u64,
    /// The offset in the encoded bytes where the transactions begin.
    tx_start_offset: u64,
}

impl Block {
    pub fn new(raw: Bytes) -> Result<Self> {
        let mut after_header = raw.slice(80..);
        let num_tx = varint_decode(&mut after_header)?;
        Ok(Self {
            raw,
            num_tx,
            tx_start_offset: 80 + varint_size(num_tx),
        })
    }

    pub fn header(&self) -> Result<BlockHeader> {
        let mut header_bytes = self.raw.slice(0..80);
        BlockHeader::from_binary(&mut header_bytes)
    }

    pub fn tx_iter(&self) -> BlockTxIterator {
        BlockTxIterator {
            buf: self.raw.slice((self.tx_start_offset as usize)..),
            num_tx: self.num_tx,
            tx_read: 0,
            offset: 0,
        }
    }

    /// Get the Genesis block for the given blockchain.
    pub fn get_genesis(blockchain_id: BlockchainId) -> Result<Block> {
        match blockchain_id {
            BlockchainId::Main => Block::from_hex("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a29ab5f49ffff001d1dac2b7c0101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000"),
            BlockchainId::Test => Block::from_hex("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4adae5494dffff001d1aa4ae180101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000"),
            BlockchainId::Stn => Block::from_hex("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4adae5494dffff001d1aa4ae180101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000"),
            BlockchainId::Regtest => Block::from_hex("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4adae5494dffff7f20020000000101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000"),
        }
    }
}

impl TryFrom<Bytes> for Block {
    type Error = Error;

    fn try_from(value: Bytes) -> Result<Self> {
        Block::new(value)
    }
}

impl FromHex for Block {
    type Error = Error;
    fn from_hex<T: AsRef<[u8]>>(hex: T) -> std::result::Result<Self, Self::Error> {
        let bytes = Vec::<u8>::from_hex(hex)?;
        let b = Bytes::from(bytes);
        Block::try_from(b)
    }
}

impl ToHex for Block {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        self.raw.encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        self.raw.encode_hex_upper()
    }
}

pub struct BlockTxIterator {
    /// A reader into the raw data, starting at the first transaction.
    buf: Bytes,
    /// The number of transactions in the block.
    num_tx: u64,
    /// The number of transactions read so far.
    tx_read: u64,
    /// The current offset into the buffer
    offset: u64,
}

impl Iterator for BlockTxIterator {
    type Item = Tx;

    fn next(&mut self) -> Option<Self::Item> {
        if self.tx_read >= self.num_tx {
            return None;
        }
        self.tx_read += 1;
        let mut b = &self.buf.slice((self.offset as usize)..)[..];
        if let Ok(tx) = Tx::from_binary(&mut b) {
            self.offset += tx.encoded_size();
            Some(tx)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::BlockchainId::{Main, Regtest, Stn, Test};

    /// Iterate through a known block.
    #[test]
    fn test_block_tx_iter() {
        let raw = std::fs::read(
            "../testdata/000000000000000006f0fc3708a93be758307b16ea39f57c7e62026355cb6bf4.bin",
        )
        .expect("Failed to read test data file");
        let block = Block::new(Bytes::from(raw)).expect("Failed to create block");
        assert_eq!(block.num_tx, 910);
        let mut tx_iter = block.tx_iter();
        let mut count = 0;
        while let Some(_) = tx_iter.next() {
            count += 1;
        }
        assert_eq!(count, 910);
    }

    /// Check that a memory copy does not occur when creating a Block, or cloning it.
    #[test]
    fn check_for_memcpy() {
        let raw = std::fs::read(
            "../testdata/000000000000000006f0fc3708a93be758307b16ea39f57c7e62026355cb6bf4.bin",
        )
        .expect("Failed to read test data file");
        let input_addr = raw.as_ptr();
        let block = Block::new(Bytes::from(raw)).expect("Failed to create block");
        let b_addr = block.raw.as_ptr();
        assert_eq!(input_addr, b_addr);
        let duplicate = block.clone();
        let d_addr = duplicate.raw.as_ptr();
        assert_eq!(input_addr, d_addr);
    }

    /// check the genesis blocks encoded
    #[test]
    fn check_genesis_blocks() {
        for i in vec![Main, Test, Stn, Regtest] {
            let genesis_block = Block::get_genesis(i).expect("Failed to get genesis block");
            let _genesis_block_hex: String = genesis_block.encode_hex();
            assert_eq!(
                genesis_block.header().expect("Failed to get header"),
                BlockHeader::get_genesis(i)
            );
        }
    }
}
