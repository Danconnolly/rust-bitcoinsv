use crate::bitcoin::{varint_decode_async, varint_encode_async, AsyncEncodable, Hash};
use async_trait::async_trait;
use std::fmt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Block locator message. This message is used to find a known block in the blockchain.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct BlockLocator {
    /// Protocol version of this node
    pub version: u32,
    /// Block hash to start after. First found will be used.
    pub block_locator_hashes: Vec<Hash>,
    /// Block hash to stop at, or none if HASH_STOP.
    pub hash_stop: Hash,
}

impl BlockLocator {
    pub const HASH_STOP: Hash = Hash::ZERO;
}

#[cfg(feature = "dev_tokio")]
#[async_trait]
impl AsyncEncodable for BlockLocator {
    async fn async_from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let version = reader.read_u32_le().await?;
        let num_hashes = varint_decode_async(reader).await? as usize;
        let mut block_locator_hashes = Vec::with_capacity(num_hashes);
        for _ in 0..num_hashes {
            block_locator_hashes.push(Hash::async_from_binary(reader).await?);
        }
        Ok(BlockLocator {
            version,
            block_locator_hashes,
            hash_stop: Hash::async_from_binary(reader).await?,
        })
    }

    async fn async_to_binary<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> crate::Result<()> {
        writer.write_u32_le(self.version).await?;
        varint_encode_async(writer, self.block_locator_hashes.len() as u64).await?;
        for hash in self.block_locator_hashes.iter() {
            hash.async_to_binary(writer).await?;
        }
        self.hash_stop.async_to_binary(writer).await?;
        Ok(())
    }

    // todo: add Encodable trait
    // fn async_size(&self) -> usize {
    //     4 + varint_size(self.block_locator_hashes.len() as u64)
    //         + self.block_locator_hashes.len() * 32
    //         + 32
    // }
}

impl fmt::Display for BlockLocator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut hashes = String::new();
        for hash in &self.block_locator_hashes {
            if hashes.is_empty() {
                hashes = format!("{}", hash);
            } else {
                hashes += &*format!(", {}", hash);
            }
        }
        write!(
            f,
            "BlockLocator(v={}, n={}, [{}], stop={})",
            self.version,
            self.block_locator_hashes.len(),
            hashes,
            self.hash_stop
        )
    }
}
