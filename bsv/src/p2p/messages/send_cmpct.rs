use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::bitcoin::Encodable;

/// Specifies whether compact blocks are supported
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct SendCmpct {
    /// Whether compact blocks may be sent
    pub enable: u8,
    /// Should always be 1
    pub version: u64,
}

impl SendCmpct {
    /// Size of the SendCmpct payload in bytes
    pub const SIZE: usize = 9;

    /// Returns whether compact blocks should be used
    pub fn use_cmpctblock(&self) -> bool {
        self.enable == 1 && self.version == 1
    }
}

#[async_trait]
impl Encodable for SendCmpct {
    async fn decode_from<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self> where Self: Sized {
        let enable = reader.read_u8().await?;
        let version = reader.read_u64_le().await?;
        Ok(SendCmpct { enable, version })
    }

    async fn encode_into<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()> {
        writer.write_u8(self.enable).await?;
        writer.write_u64_le(self.version).await?;
        Ok(())
    }

    fn size(&self) -> usize {
        SendCmpct::SIZE
    }
}