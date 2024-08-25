use async_trait::async_trait;
use hex::{FromHex, ToHex};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::bitcoin::Encodable;
use crate::bitcoin::hash::Hash;
use crate::bitcoin::params::BlockchainId;

/// The BlockHash is used to identify block headers and enforce proof of work.
pub type BlockHash = Hash;
/// The MerkleRoot is the root of the merkle tree of this block's transaction hashes.
pub type MerkleRoot = Hash;

/// BlockHeaders are linked to together to form a blockchain.
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct BlockHeader {
    /// Block version.
    pub version: u32,
    /// Hash of the previous block header.
    pub prev_hash: BlockHash,
    /// Root of the merkle tree of this block's transaction hashes.
    pub merkle_root: MerkleRoot,
    /// Timestamp when this block was created as recorded by the miner.
    pub timestamp: u32,
    /// Target difficulty bits.
    pub bits: u32,
    /// Nonce used to mine the block.
    pub nonce: u32,
}

impl BlockHeader {
    /// Size of the BlockHeader in bytes
    pub const SIZE: usize = 80;
    pub const HEX_SIZE: usize = BlockHeader::SIZE * 2;

    /// Calculates the hash for this block header
    pub fn hash(&self) -> BlockHash {
        let v = self.to_binary_buf().unwrap();
        Hash::sha256d(&v)
    }

    /// Get the Genesis BlockHeader for the given chain.
    pub fn get_genesis(block_chain: BlockchainId) -> BlockHeader {
        match block_chain {
            BlockchainId::Main => BlockHeader::from_hex("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a29ab5f49ffff001d1dac2b7c").unwrap(),
            BlockchainId::Test => BlockHeader::from_hex("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4adae5494dffff001d1aa4ae18").unwrap(),
            BlockchainId::Stn => BlockHeader::from_hex("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4adae5494dffff001d1aa4ae18").unwrap(),
            BlockchainId::Regtest => BlockHeader::from_hex("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4adae5494dffff7f2002000000").unwrap(),
        }
    }
}

#[async_trait]
impl Encodable for BlockHeader {
    async fn from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::BsvResult<Self> where Self: Sized {
        Ok(BlockHeader {
            version: reader.read_u32_le().await?,
            prev_hash: Hash::from_binary(reader).await?,
            merkle_root: Hash::from_binary(reader).await?,
            timestamp: reader.read_u32_le().await?,
            bits: reader.read_u32_le().await?,
            nonce: reader.read_u32_le().await?,
        })
    }

    async fn to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::BsvResult<()> {
        writer.write_u32_le(self.version).await?;
        self.prev_hash.to_binary(writer).await?;
        self.merkle_root.to_binary(writer).await?;
        writer.write_u32_le(self.timestamp).await?;
        writer.write_u32_le(self.bits).await?;
        writer.write_u32_le(self.nonce).await?;
        Ok(())
    }

    fn size(&self) -> usize {
        BlockHeader::SIZE
    }
}

impl FromHex for BlockHeader {
    type Error = crate::BsvError;
    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let bytes = Vec::<u8>::from_hex(hex)?;
        BlockHeader::from_binary_buf(bytes.as_slice())
    }
}

impl ToHex for BlockHeader {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        let bytes = self.to_binary_buf().unwrap();
        bytes.encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        let bytes = self.to_binary_buf().unwrap();
        bytes.encode_hex_upper()
    }
}

#[cfg(test)]
mod tests {
    use hex::FromHex;
    use super::*;

    /// Read a block header from a byte array and check it
    #[test]
    fn block_header_read() {
        let (block_header_bin, block_header_hash) = get_block_header824962();
        let block_header = BlockHeader::from_binary_buf(block_header_bin.as_slice()).unwrap();
        assert_eq!(block_header.version, 609435648);
        assert_eq!(block_header.hash(), block_header_hash);
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

    #[test]
    fn from_hex_non_async() {
        let bh = BlockHeader::from_hex("00405324d8facaf19ce3efc5f6b3fbdc1cb1f5369a56c3de3e50280300000000000000002742bdb5930e5bf24be6e7521ceeecf6d3199871e2a6438f54cb5fd95d3f5139a38d90653c5808186eac9b4c").unwrap();
        assert_eq!(bh.version, 609435648);
    }

    #[test]
    fn from_hex_async() {
        let bh = BlockHeader::from_hex("00405324d8facaf19ce3efc5f6b3fbdc1cb1f5369a56c3de3e50280300000000000000002742bdb5930e5bf24be6e7521ceeecf6d3199871e2a6438f54cb5fd95d3f5139a38d90653c5808186eac9b4c").unwrap();
        assert_eq!(bh.version, 609435648);
    }

    #[test]
    fn check_hex_encode() {
        let o = "00405324d8facaf19ce3efc5f6b3fbdc1cb1f5369a56c3de3e50280300000000000000002742bdb5930e5bf24be6e7521ceeecf6d3199871e2a6438f54cb5fd95d3f5139a38d90653c5808186eac9b4c";
        let bh = BlockHeader::from_hex(o).unwrap();
        let s = bh.encode_hex::<String>();
        assert_eq!(s, o);
    }

    // check that the genesis blocks have been correctly implemented
    #[test]
    fn check_genesis() {
        let hdr = BlockHeader::get_genesis(BlockchainId::Main);
        assert_eq!(hdr.hash(), BlockHash::from_hex("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f").unwrap());
        let hdr = BlockHeader::get_genesis(BlockchainId::Test);
        assert_eq!(hdr.hash(), BlockHash::from_hex("000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943").unwrap());
        let hdr = BlockHeader::get_genesis(BlockchainId::Stn);
        assert_eq!(hdr.hash(), BlockHash::from_hex("000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943").unwrap());
        let hdr = BlockHeader::get_genesis(BlockchainId::Regtest);
        assert_eq!(hdr.hash(), BlockHash::from_hex("0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206").unwrap());
    }
}