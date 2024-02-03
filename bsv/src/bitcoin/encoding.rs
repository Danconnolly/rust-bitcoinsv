use async_trait::async_trait;
use byteorder::{ReadBytesExt, WriteBytesExt};
use futures::executor::block_on;
use tokio::io::{AsyncRead, AsyncWrite};

// Bitcoin encoding standard binary serialization traits.

/// Encode & decode Bitcoin data structures synchronously.
pub trait Encodable {
    /// Decode data structure from a reader.
    fn decode<R: ReadBytesExt + Send>(reader: &mut R) -> crate::Result<Self>
    where
        Self: Sized;

    /// Encode data structure to a writer.
    fn encode_into<W: WriteBytesExt + Send>(&self, writer: &mut W) -> crate::Result<()>;

    /// Encode a data structure into a new vector.
    fn encode(&self) -> crate::Result<Vec<u8>> {
        let mut v = Vec::with_capacity(self.size());
        self.encode_into(&mut v)?;
        Ok(v)
    }

    /// Return the size of the serialized form.
    // It is vital that implementations of this function use a method that does not just serialize the object
    // and count the bytes. This is because this function is used to determine the size of the buffer to allocate
    // for the serialization.
    fn size(&self) -> usize;
}

/// Encode & decode Bitcoin data structures asynchronously.
///
/// These functions could have the same name as in the Encodable trait, but that would require
/// differentiating between the traits throughout the code.
#[async_trait]
pub trait AsyncEncodable {
    /// Decode data structure from a reader.
    async fn decode_async<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self>
    where
        Self: Sized;

    /// Encode data structure to a writer.
    async fn encode_into_async<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()>;

    /// Return the size of the serialized form.
    // It is vital that implementations of this function use a method that does not just serialize the object
    // and count the bytes. This is because this function is used to determine the size of the buffer to allocate
    // for the serialization.
    fn size(&self) -> usize;

    /// Decode data structure from a buffer.
    fn decode_from_buf(buf: &[u8]) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let mut reader = std::io::Cursor::new(buf);
        block_on(Self::decode_async(&mut reader))
    }

    /// Encode data structure to a new buffer.
    fn encode_into_buf(&self) -> crate::Result<Vec<u8>> {
        let mut v = Vec::with_capacity(self.size());
        block_on(self.encode_into_async(&mut v))?;
        Ok(v)
    }
}
