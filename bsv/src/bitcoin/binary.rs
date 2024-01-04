use std::io::Cursor;
use async_trait::async_trait;
use futures::executor::block_on;
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

    /// Deserializes an object from a byte array.
    ///
    /// A non-async version of `read` to read from a byte slice.
    fn read_from_buf(buf: &[u8]) -> crate::Result<Self>
    where
        Self: Sized,
    {
        block_on(Self::read(&mut Cursor::new(buf)))
    }
}

