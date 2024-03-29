use async_trait::async_trait;
use futures::executor::block_on;
use tokio::io::{AsyncRead, AsyncWrite};

// Bitcoin encoding standard binary serialization traits.

/// Read & write Bitcoin data structures to and from binary in Bitcoin encoding format.
///
/// This trait uses function names that are consistent with the Rust serde standard names. Note that
/// it doesnt really make sense to separate the decode and encoding functions into separate traits.
#[async_trait]
pub trait Encodable {
    /// Read the data structure from a reader.
    async fn from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self>
    where
        Self: Sized;

    /// Write the data structure to a writer.
    async fn to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()>;

    /// Return the size of the serialized form.
    // It is vital that implementations of this function use a method that does not just serialize the object
    // and count the bytes. This is because this function is used to determine the size of the buffer to allocate
    // for the serialization.
    fn size(&self) -> usize;

    /// Read the data structure from a buffer.
    ///
    /// This is a convenience function that wraps the `from_binary` function.
    fn from_binary_buf(buf: &[u8]) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let mut reader = std::io::Cursor::new(buf);
        block_on(Self::from_binary(&mut reader))
    }

    /// Write the data structure to a new buffer.
    ///
    /// This is a convenience function that wraps the `to_binary` function.
    fn to_binary_buf(&self) -> crate::Result<Vec<u8>> {
        let mut v = Vec::with_capacity(self.size());
        block_on(self.to_binary(&mut v))?;
        Ok(v)
    }
}
