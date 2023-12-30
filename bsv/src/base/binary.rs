use tokio::io::{AsyncRead, AsyncWrite};

/// Custom binary serialization traits.

pub trait Encodable {
    /// Read object from an async reader.
    async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> crate::Result<Self>
    where
        Self: Sized;

    /// Write object to an async writer.
    async fn write<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> crate::Result<()>;
}

