use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::bitcoin::{AsyncEncodable, varint_decode, varint_encode, varint_size};

// Message rejection error codes
pub const REJECT_MALFORMED: u8 = 0x01;
pub const REJECT_INVALID: u8 = 0x10;
pub const REJECT_OBSOLETE: u8 = 0x11;
pub const REJECT_DUPLICATE: u8 = 0x12;
pub const REJECT_NONSTANDARD: u8 = 0x40;
pub const REJECT_DUST: u8 = 0x41;
pub const REJECT_INSUFFICIENT_FEE: u8 = 0x42;
pub const REJECT_CHECKPOINT: u8 = 0x43;

/// Rejected message
#[derive(Default, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Reject {
    /// Type of message rejected
    pub message: String,
    /// Error code
    pub code: u8,
    /// Reason for rejection
    pub reason: String,
    /// Optional extra data that may be present for some rejections
    ///
    /// Currently this is only a 32-byte hash of the block or transaction if applicable.
    pub data: Vec<u8>,
}

#[async_trait]
impl AsyncEncodable for Reject {
    async fn async_from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self> where Self: Sized {
        let str_size = varint_decode(reader).await? as usize;
        let mut str_bytes = vec![0; str_size];
        reader.read_exact(&mut str_bytes).await?;
        let message = String::from_utf8(str_bytes)?;
        let code = reader.read_u8().await?;
        let reason_size = varint_decode(reader).await? as usize;
        let mut reason_bytes = vec![0; reason_size];
        reader.read_exact(&mut reason_bytes).await?;
        let reason = String::from_utf8(reason_bytes)?;
        let mut data = vec![];
        if message == "block".to_string() || message == "tx".to_string() {
            data = vec![0_u8; 32];
            reader.read_exact(&mut data).await?;
        }
        Ok(Reject {
            message,
            code,
            reason,
            data,
        })
    }

    async fn async_to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()> {
        varint_encode(writer, self.message.len() as u64).await?;
        writer.write_all(self.message.as_bytes()).await?;
        writer.write_u8(self.code).await?;
        varint_encode(writer, self.reason.len() as u64).await?;
        writer.write_all(self.reason.as_bytes()).await?;
        if self.message == "block".to_string() || self.message == "tx".to_string() {
            writer.write_all(&self.data).await?;
        }
        Ok(())
    }

    fn async_size(&self) -> usize {
        let mut size = varint_size(self.message.len() as u64) + self.message.len();
        size += 1;
        size += varint_size(self.reason.len() as u64) + self.reason.len();
        if self.message == "block".to_string() || self.message == "tx".to_string() {
            size += 32;
        }
        size
    }
}
