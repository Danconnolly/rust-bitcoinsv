use crate::Result;
use bytes::{Buf, BufMut};

/// Read & write Bitcoin data structures to and from binary in Bitcoin encoding format.
pub trait Encodable {
    /// Read the data structure from a buffer.
    fn from_binary(buffer: &mut dyn Buf) -> Result<Self>
    where
        Self: Sized;

    /// Write the data structure to a buffer.
    fn to_binary(&self, buffer: &mut dyn BufMut) -> Result<()>;

    /// Return the size of the encoded form.
    // It is vital (for efficiency) that implementations of this function use a method that does not just encode the object
    // and count the bytes. This is because this function can be used to determine the size of the buffer to allocate
    // for the encoding.
    fn encoded_size(&self) -> u64;
}
