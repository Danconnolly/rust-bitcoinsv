use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

/// Custom binary serialization traits.

#[async_trait]
pub trait Encodable {
    /// Read object from an async reader.
    async fn read<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self>
    where
        Self: Sized;

    /// Write object to an async writer.
    async fn write<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()>;
}

