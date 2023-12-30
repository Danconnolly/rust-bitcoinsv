use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
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
    pub fn binary_size(&self) -> usize {
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
    async fn read<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<BlockHeader> {
        Ok(BlockHeader {
            version: reader.read_u32_le().await?,
            prev_hash: Hash::read(reader).await?,
            merkle_root: Hash::read(reader).await?,
            timestamp: reader.read_u32_le().await?,
            bits: reader.read_u32_le().await?,
            nonce: reader.read_u32_le().await?,
        })
    }

    async fn write<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()> {
        writer.write_u32_le(self.version).await?;
        self.prev_hash.write(writer).await?;
        self.merkle_root.write(writer).await?;
        writer.write_u32_le(self.timestamp).await?;
        writer.write_u32_le(self.bits).await?;
        writer.write_u32_le(self.nonce).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hex::FromHex;
    use super::*;

    /// Read a block header from a byte array and check it
    #[tokio::test]
    async fn block_header_read() {
        let (block_header_bin, block_header_hash) = get_block_header824962();
        let mut cursor = std::io::Cursor::new(&block_header_bin);
        let block_header = BlockHeader::read(&mut cursor).await.unwrap();
        assert_eq!(block_header.version, 609435648);
        assert_eq!(block_header.hash().await, block_header_hash);
        assert_eq!(block_header.nonce, 1285270638);
        assert_eq!(block_header.bits, 0x1808583c);
        assert_eq!(block_header.merkle_root, Hash::from_hex("39513f5dd95fcb548f43a6e2719819d3f6ecee1c52e7e64bf25b0e93b5bd4227").unwrap());
        assert_eq!(block_header.timestamp, 1703972259);
        assert_eq!(block_header.prev_hash, Hash::from_hex("00000000000000000328503edec3569a36f5b11cdcfbb3f6c5efe39cf1cafad8").unwrap());
    }

    fn get_block_header824962() -> (Vec<u8>, BlockHash) {
        (
            Vec::from_hex("00405324d8facaf19ce3efc5f6b3fbdc1cb1f5369a56c3de3e50280300000000000000002742bdb5930e5bf24be6e7521ceeecf6d3199871e2a6438f54cb5fd95d3f5139a38d90653c5808186eac9b4c").unwrap(),
            Hash::from_hex("000000000000000001749126813c455cabd41bb80fdfc1833ffe09deacb91967").unwrap()
        )
    }
}