use std::cmp::min;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use crate::base::binary::Encodable;
use crate::Hash;

pub type BlockHash = Hash;
pub type MerkleRoot = Hash;

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct BlockHeader {
    /// Block version
    pub version: u32,
    /// Hash of the previous block
    pub prev_hash: BlockHash,
    /// Root of the merkle tree of this block's transaction hashes
    pub merkle_root: MerkleRoot,
    /// Timestamp when this block was created as recorded by the miner
    pub timestamp: u32,
    /// Target difficulty bits
    pub bits: u32,
    /// Nonce used to mine the block
    pub nonce: u32,
}

impl BlockHeader {
    /// Size of the BlockHeader in bytes
    pub const BINARY_SIZE: usize = 80;
    pub const HEX_SIZE: usize = BlockHeader::BINARY_SIZE * 2;

    /// Returns the size of the block header in bytes
    pub fn size(&self) -> usize {
        BlockHeader::BINARY_SIZE
    }

    /// Calculates the hash for this block header
    pub async fn hash(&self) -> BlockHash {
        let mut v = Vec::with_capacity(80);
        self.write(&mut v).await.unwrap();
        Hash::sha256d(&v)
    }
}

#[async_trait]
impl Encodable for BlockHeader {
    async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> crate::Result<BlockHeader> {
        todo!()
    }

    async fn write<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> crate::Result<()> {
        todo!()
    }
}