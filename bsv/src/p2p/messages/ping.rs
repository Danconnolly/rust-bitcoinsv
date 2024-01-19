use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io;
use std::io::{Read, Write};
use crate::bitcoin::Encodable;

/// Ping or pong message
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct Ping {
    /// Unique identifier nonce
    pub nonce: u64,
}

impl Ping {
    /// Size of the ping or pong payload in bytes
    pub const SIZE: usize = 8;

    pub fn new(nonce: u64) -> Ping {
        Ping { nonce }
    }
}

impl Encodable for Ping {
    fn decode<R: ReadBytesExt + Send>(reader: &mut R) -> crate::Result<Self> where Self: Sized {
        let nonce = reader.read_u64::<LittleEndian>()?;
        Ok(Ping { nonce })
    }

    fn encode_into<W: WriteBytesExt + Send>(&self, writer: &mut W) -> crate::Result<()> {
        writer.write_u64::<LittleEndian>(self.nonce)?;
        Ok(())
    }

    fn size(&self) -> usize {
        Self::SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;
    use std::io::Cursor;

    #[test]
    fn read_bytes() {
        let b = hex::decode("86b19332b96c657d".as_bytes()).unwrap();
        let f = Ping::decode(&mut Cursor::new(&b)).unwrap();
        assert_eq!(f.nonce, 9035747770062057862);
    }

    #[test]
    fn write_read() {
        let p = Ping { nonce: 13579 };
        let v = p.encode().unwrap();
        assert_eq!(v.len(), p.size());
        assert_eq!(Ping::decode(&mut Cursor::new(&v)).unwrap(), p);
    }
}
