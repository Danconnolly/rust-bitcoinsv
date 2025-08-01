use crate::bitcoin::hash::Hash;
use crate::bitcoin::params::BlockchainId;
use crate::bitcoin::Encodable;
use crate::Error;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use hex::{FromHex, ToHex};
use std::fmt::{Debug, Display, Formatter};

/// The BlockHash is used to identify block headers and implement proof of work.
pub type BlockHash = Hash;
/// The MerkleRoot is the root of the merkle tree of this block's transaction hashes.
pub type MerkleRoot = Hash;

/// BlockHeaders are linked together to form a blockchain.
///
/// This implementation stores the encoded form and extracts fields when they are requested.
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BlockHeader {
    pub raw: Bytes,
}

impl BlockHeader {
    /// Size of the BlockHeader in bytes
    pub const SIZE: u64 = 80;
    pub const HEX_SIZE: u64 = BlockHeader::SIZE * 2;

    /// Block hash.
    pub fn hash(&self) -> BlockHash {
        BlockHash::sha256d(self.raw.as_ref())
    }

    /// Block version.
    pub fn version(&self) -> u32 {
        let mut slice = &self.raw[0..4];
        slice.get_u32_le()
    }

    /// Hash of the previous block header.
    pub fn prev_hash(&self) -> BlockHash {
        let slice = &self.raw[4..36];
        BlockHash::from(slice)
    }

    /// Root of the merkle tree of this block's transaction hashes.
    pub fn merkle_root(&self) -> MerkleRoot {
        let slice = &self.raw[36..68];
        MerkleRoot::from(slice)
    }

    /// Timestamp when this block was created as recorded by the miner.
    pub fn timestamp(&self) -> u32 {
        let mut slice = &self.raw[68..72];
        slice.get_u32_le()
    }

    /// Target difficulty bits.
    pub fn bits(&self) -> u32 {
        let mut slice = &self.raw[72..76];
        slice.get_u32_le()
    }

    /// Block difficulty
    pub fn difficulty(&self) -> f64 {
        let bits = self.bits();
        let exponent = bits >> 24;
        let coefficient = bits & 0xFFFFFF;
        let target = coefficient as f64 * 256f64.powi(exponent as i32 - 3);
        let max_target = 0xFFFF as f64 * 256f64.powi(0x1D - 3);
        max_target / target
    }

    /// Nonce used to mine the block.
    pub fn nonce(&self) -> u32 {
        let mut slice = &self.raw[76..80];
        slice.get_u32_le()
    }

    /// Create a block header from a slice.
    ///
    /// Note that From::Bytes is better if you already have a Bytes.
    pub fn from_slice(slice: &[u8]) -> Self {
        let v = slice.to_vec();
        let raw = Bytes::from(v);
        Self { raw }
    }

    /// Get the Genesis BlockHeader for the given chain.
    pub fn get_genesis(block_chain: BlockchainId) -> BlockHeader {
        match block_chain {
            BlockchainId::Main => BlockHeader::from_hex("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a29ab5f49ffff001d1dac2b7c").expect("Failed to decode main genesis header"),
            BlockchainId::Test => BlockHeader::from_hex("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4adae5494dffff001d1aa4ae18").expect("Failed to decode test genesis header"),
            BlockchainId::Stn => BlockHeader::from_hex("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4adae5494dffff001d1aa4ae18").expect("Failed to decode stn genesis header"),
            BlockchainId::Regtest => BlockHeader::from_hex("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4adae5494dffff7f2002000000").expect("Failed to decode regtest genesis header"),
        }
    }
}

impl Encodable for BlockHeader {
    fn from_binary(buffer: &mut dyn Buf) -> crate::Result<Self> {
        if buffer.remaining() < Self::SIZE as usize {
            Err(Error::DataTooSmall)
        } else {
            Ok(Self {
                raw: buffer.copy_to_bytes(Self::SIZE as usize),
            })
        }
    }

    fn to_binary(&self, buffer: &mut dyn BufMut) -> crate::Result<()> {
        buffer.put_slice(&self.raw);
        Ok(())
    }

    fn encoded_size(&self) -> u64 {
        Self::SIZE
    }
}

impl FromHex for BlockHeader {
    type Error = Error;
    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let bytes = Vec::<u8>::from_hex(hex)?;
        let mut b = Bytes::from(bytes);
        BlockHeader::from_binary(&mut b)
    }
}

impl ToHex for BlockHeader {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        let mut bytes = BytesMut::with_capacity(BlockHeader::SIZE as usize);
        self.to_binary(&mut bytes)
            .expect("Failed to encode header to binary");
        bytes.encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        let mut bytes = BytesMut::with_capacity(BlockHeader::SIZE as usize);
        self.to_binary(&mut bytes)
            .expect("Failed to encode header to binary");
        bytes.encode_hex_upper()
    }
}

impl From<Bytes> for BlockHeader {
    fn from(value: Bytes) -> Self {
        Self { raw: value }
    }
}

impl From<BlockHeader> for Vec<u8> {
    fn from(value: BlockHeader) -> Self {
        value.raw.to_vec()
    }
}

impl Display for BlockHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ hash: {}, prev_block: {}, v: {}, merkle: {}, ts: {}, bits: {:x}, nonce: {}, difficulty: {:.8} }}",
            self.hash(),
            self.prev_hash(),
            self.version(),
            self.merkle_root(),
            self.timestamp(),
            self.bits(),
            self.nonce(),
            self.difficulty()
        )
    }
}

impl Debug for BlockHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockHeader")
            .field("hash", &self.hash())
            .field("version", &self.version())
            .field("prev_hash", &self.prev_hash())
            .field("merkle_root", &self.merkle_root())
            .field("timestamp", &self.timestamp())
            .field("bits", &self.bits())
            .field("nonce", &self.nonce())
            .field("difficulty", &self.difficulty())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex::FromHex;

    /// Read a block header from a byte array and check it
    #[test]
    fn block_header_read() {
        let (block_header_bin, block_header_hash) = get_block_header824962();
        let mut bh_bytes = Bytes::from(block_header_bin);
        let block_header =
            BlockHeader::from_binary(&mut bh_bytes).expect("Failed to decode test block header");
        assert_eq!(block_header.version(), 609435648);
        assert_eq!(block_header.hash(), block_header_hash);
        assert_eq!(block_header.nonce(), 1285270638);
        assert_eq!(block_header.bits(), 0x1808583c);
        assert_eq!(
            block_header.merkle_root(),
            Hash::from_hex("39513f5dd95fcb548f43a6e2719819d3f6ecee1c52e7e64bf25b0e93b5bd4227")
                .expect("Failed to decode test merkle root")
        );
        assert_eq!(block_header.timestamp(), 1703972259);
        assert_eq!(
            block_header.prev_hash(),
            Hash::from_hex("00000000000000000328503edec3569a36f5b11cdcfbb3f6c5efe39cf1cafad8")
                .expect("Failed to decode test prev hash")
        );
        assert_eq!(block_header.difficulty(), 131760206200.85753);
    }

    fn get_block_header824962() -> (Vec<u8>, BlockHash) {
        (
            Vec::from_hex("00405324d8facaf19ce3efc5f6b3fbdc1cb1f5369a56c3de3e50280300000000000000002742bdb5930e5bf24be6e7521ceeecf6d3199871e2a6438f54cb5fd95d3f5139a38d90653c5808186eac9b4c").expect("Failed to decode test header data"),
            Hash::from_hex("000000000000000001749126813c455cabd41bb80fdfc1833ffe09deacb91967").expect("Failed to decode test header hash")
        )
    }

    #[test]
    fn check_hex_encode() {
        let o = "00405324d8facaf19ce3efc5f6b3fbdc1cb1f5369a56c3de3e50280300000000000000002742bdb5930e5bf24be6e7521ceeecf6d3199871e2a6438f54cb5fd95d3f5139a38d90653c5808186eac9b4c";
        let bh = BlockHeader::from_hex(o).expect("Failed to decode test header from hex");
        let s = bh.encode_hex::<String>();
        assert_eq!(s, o);
    }

    // check that the genesis blocks have been correctly implemented
    #[test]
    fn check_genesis() {
        let hdr = BlockHeader::get_genesis(BlockchainId::Main);
        assert_eq!(
            hdr.hash(),
            BlockHash::from_hex("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f")
                .expect("Failed to decode main genesis hash")
        );
        let hdr = BlockHeader::get_genesis(BlockchainId::Test);
        assert_eq!(
            hdr.hash(),
            BlockHash::from_hex("000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943")
                .expect("Failed to decode test genesis hash")
        );
        let hdr = BlockHeader::get_genesis(BlockchainId::Stn);
        assert_eq!(
            hdr.hash(),
            BlockHash::from_hex("000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943")
                .expect("Failed to decode test genesis hash")
        );
        let hdr = BlockHeader::get_genesis(BlockchainId::Regtest);
        assert_eq!(
            hdr.hash(),
            BlockHash::from_hex("0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206")
                .expect("Failed to decode regtest genesis hash")
        );
    }
}
