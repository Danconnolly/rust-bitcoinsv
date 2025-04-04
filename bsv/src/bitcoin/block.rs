use crate::bitcoin::{varint_decode, varint_size, BlockHeader, Encodable, Tx};
use crate::Result;
use bytes::Bytes;

/// Contains a full block from the blockchain.
///
/// This can get large, a 4GB block will consume at least 4GB of RAM.
#[derive(Clone, Debug)]
pub struct Block {
    /// The encoded block.
    raw: Bytes,
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
        BlockHeader::from_binary(&mut header_bytes).map_err(From::from)
    }

    pub fn tx_iter(&self) -> BlockTxIterator {
        BlockTxIterator {
            buf: self.raw.slice((self.tx_start_offset as usize)..),
            num_tx: self.num_tx,
            tx_read: 0,
            offset: 0,
        }
    }
}

impl From<Bytes> for Block {
    fn from(value: Bytes) -> Self {
        Block::new(value).unwrap()
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

    /// Iterate through a known block.
    #[test]
    fn test_block_tx_iter() {
        let raw = std::fs::read(
            "../testdata/000000000000000006f0fc3708a93be758307b16ea39f57c7e62026355cb6bf4.bin",
        )
        .unwrap();
        let block = Block::new(Bytes::from(raw)).unwrap();
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
        .unwrap();
        let input_addr = raw.as_ptr();
        let block = Block::new(Bytes::from(raw)).unwrap();
        let b_addr = block.raw.as_ptr();
        assert_eq!(input_addr, b_addr);
        let duplicate = block.clone();
        let d_addr = duplicate.raw.as_ptr();
        assert_eq!(input_addr, d_addr);
    }
}
