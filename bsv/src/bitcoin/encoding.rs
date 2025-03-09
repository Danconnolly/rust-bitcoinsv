use crate::Result;
#[cfg(feature="dev_tokio")]
use async_trait::async_trait;
use bytes::{Buf, BufMut};
#[cfg(feature="dev_tokio")]
use futures::executor::block_on;
#[cfg(feature="dev_tokio")]
use tokio::io::{AsyncRead, AsyncWrite};

/// Read & write Bitcoin data structures to and from binary in Bitcoin encoding format.
///
/// This is the non-async trait for reading and writing to buffers. See [AsyncEncodable] for the
/// async trait to read and write with truly async sources.
pub trait Encodable {
    /// Read the data structure from a buffer.
    fn from_binary(buffer: &mut dyn Buf) -> Result<Self> where Self: Sized;

    /// Write the data structure to a buffer.
    fn to_binary(&self, buffer: &mut dyn BufMut) -> Result<()>;

    /// Return the size of the serialized form.
    // It is vital (for efficiency) that implementations of this function use a method that does not just serialize the object
    // and count the bytes. This is because this function is used to determine the size of the buffer to allocate
    // for the serialization.
    fn size(&self) -> u64;
}

/// Asynchronously read & write Bitcoin data structures to and from binary in Bitcoin encoding format.
///
/// This trait includes standard implementations to read from a buffer instead of from a truly
/// asynchronous source but these should only be used if the [Encodable] trait is not implemented.
///
/// For a discussion on async versus non-async, see [Encodable].
#[cfg(feature="dev_tokio")]
#[async_trait]
pub trait AsyncEncodable {
    /// Read the data structure from an async reader.
    async fn async_from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> Result<Self>
    where
        Self: Sized;

    /// Write the data structure to an async writer.
    async fn async_to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()>;

    /// Return the size of the serialized form.
    // It is vital that implementations of this function use a method that does not just serialize the object
    // and count the bytes. This is because this function is used to determine the size of the buffer to allocate
    // for the serialization.
    fn async_size(&self) -> u64;

    /// Read the data structure from a buffer.
    ///
    /// This is a convenience function that wraps the `from_binary` function.
    fn from_binary_buf(buf: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let mut reader = std::io::Cursor::new(buf);
        block_on(Self::async_from_binary(&mut reader))
    }

    /// Write the data structure to a new buffer.
    ///
    /// This is a convenience function that wraps the `to_binary` function.
    fn to_binary_buf(&self) -> Result<Vec<u8>> {
        let mut v = Vec::with_capacity(self.async_size());
        block_on(self.async_to_binary(&mut v))?;
        Ok(v)
    }
}
