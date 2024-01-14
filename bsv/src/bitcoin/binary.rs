use byteorder::{ReadBytesExt, WriteBytesExt};

/// Bitcoin encoding standard binary serialization traits.

pub trait Encodable {
    /// Decode object from a reader.
    fn decode<R: ReadBytesExt + Send>(reader: &mut R) -> crate::Result<Self>
    where
        Self: Sized;

    /// Encode object to a writer.
    fn encode_into<W: WriteBytesExt + Send>(&self, writer: &mut W) -> crate::Result<()>;

    /// Encode an object into a new vector.
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

