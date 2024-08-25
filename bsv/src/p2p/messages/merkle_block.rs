use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::bitcoin::{BlockHeader, Encodable, varint_decode, varint_encode, varint_size};
use crate::bitcoin::hash::Hash;


/// A block header and partial merkle tree for SPV nodes to validate transactions
#[derive(Default, PartialEq, Eq, Hash, Clone, Debug)]
pub struct MerkleBlock {
    /// Block header
    pub header: BlockHeader,
    /// Number of transactions in the block
    pub total_transactions: u32,
    /// Hashes in depth-first order
    pub hashes: Vec<Hash>,
    /// Bit vector used to assign hashes to nodes in the partial merkle tree
    pub flags: Vec<u8>,
}

#[async_trait]
impl Encodable for MerkleBlock {
    async fn from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::BsvResult<Self> where Self: Sized {
        let header = BlockHeader::from_binary(reader).await?;
        let total_transactions = reader.read_u32_le().await?;
        let num_hashes = varint_decode(reader).await? as usize;
        let mut hashes = Vec::with_capacity(num_hashes);
        for _ in 0..num_hashes {
            hashes.push(Hash::from_binary(reader).await?);
        }
        let num_flags = varint_decode(reader).await? as usize;
        let mut flags = Vec::with_capacity(num_flags);
        for _ in 0..num_flags {
            flags.push(reader.read_u8().await?);
        }
        Ok(MerkleBlock {
            header,
            total_transactions,
            hashes,
            flags,
        })
    }

    async fn to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::BsvResult<()> {
        self.header.to_binary(writer).await?;
        writer.write_u32_le(self.total_transactions).await?;
        varint_encode(writer, self.hashes.len() as u64).await?;
        for hash in self.hashes.iter() {
            hash.to_binary(writer).await?;
        }
        varint_encode(writer, self.flags.len() as u64).await?;
        for flag in self.flags.iter() {
            writer.write_u8(*flag).await?;
        }
        Ok(())
    }

    fn size(&self) -> usize {
        self.header.size() + 4 + varint_size(self.hashes.len() as u64) + self.hashes.len() * 32 +
            varint_size(self.flags.len() as u64) + self.flags.len()
    }
}
