use byteorder::{ReadBytesExt, WriteBytesExt};

/// Bitcoin encoding standard binary serialization traits.

pub trait Encodable {
    /// Read object from an reader.
    fn read<R: ReadBytesExt + Send>(reader: &mut R) -> crate::Result<Self>
    where
        Self: Sized;

    /// Write object to a writer.
    fn write<W: WriteBytesExt + Send>(&self, writer: &mut W) -> crate::Result<()>;

    /// Return the size of the serialized form.
    // It is vital that implementations of this function use a method that does not just serialize the object
    // and count the bytes. This is because this function is used to determine the size of the buffer to allocate
    // for the serialization.
    fn size(&self) -> usize;
}

