use crate::{Error, Result};
use std::fmt;
use std::str;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::bitcoin::Encodable;

// based on code imported from rust-sv but substantially modified

// todo: extended message sizes in BSV

/// Header that begins all messages
#[derive(Default, PartialEq, Eq, Hash, Clone)]
pub struct P2PMessageHeader {
    /// Magic bytes indicating the network type
    pub magic: [u8; 4],
    /// Command name
    pub command: [u8; 12],
    /// Payload size
    pub payload_size: u32,
    /// First 4 bytes of SHA256(SHA256(payload))
    pub checksum: [u8; 4],
}

impl P2PMessageHeader {
    /// Size of the message header in bytes
    pub const SIZE: usize = 24;

    /// Checks if the header is valid
    ///
    /// `magic` - Expected magic bytes for the network
    /// `max_size` - Max size in bytes for the payload
    pub fn validate(&self, magic: [u8; 4], max_size: u64) -> Result<()> {
        if self.magic != magic {
            let msg = format!("Bad magic: {:?}", self.magic);
            return Err(Error::BadData(msg));
        }
        if self.payload_size as u64 > max_size {
            let msg = format!("Bad size: {:?}", self.payload_size);
            return Err(Error::BadData(msg));
        }
        Ok(())
    }
}

#[async_trait]
impl Encodable for P2PMessageHeader {
    async fn read<R: AsyncRead + Unpin + Send>(reader: &mut R) -> Result<Self> where Self: Sized {
        let mut magic = vec![0u8; 4];
        let _ = reader.read_exact(magic.as_mut()).await?;
        let mut command = vec![0u8; 12];
        let _ = reader.read_exact(command.as_mut()).await?;
        let payload_size = reader.read_u32_le().await?;
        let mut checksum = vec![0u8; 4];
        let _ = reader.read_exact(checksum.as_mut()).await?;
        Ok(P2PMessageHeader { magic: magic.try_into().unwrap(), command: command.try_into().unwrap(),
            payload_size, checksum: checksum.try_into().unwrap(), })
    }

    async fn write<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        writer.write(&self.magic).await?;
        writer.write(&self.command).await?;
        writer.write_u32_le(self.payload_size).await?;
        writer.write(&self.checksum).await?;
        Ok(())
    }

    fn size(&self) -> usize {
        P2PMessageHeader::SIZE
    }
}

// Prints so the command is easier to read
impl fmt::Debug for P2PMessageHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let command = match str::from_utf8(&self.command) {
            Ok(s) => s.to_string(),
            Err(_) => format!("Not Ascii ({:?})", self.command),
        };
        write!(
            f,
            "Header {{ magic: {:?}, command: {:?}, payload_size: {}, checksum: {:?} }}",
            self.magic, command, self.payload_size, self.checksum
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;
    use std::io::Cursor;

    #[tokio::test]
    async fn read_bytes() {
        let b = hex::decode("f9beb4d976657273696f6e00000000007a0000002a1957bb".as_bytes()).unwrap();
        let h = P2PMessageHeader::read(&mut Cursor::new(&b)).await.unwrap();
        assert_eq!(h.magic, [0xf9, 0xbe, 0xb4, 0xd9]);
        assert_eq!(h.command, *b"version\0\0\0\0\0");
        assert_eq!(h.payload_size, 122);
        assert_eq!(h.checksum, [0x2a, 0x19, 0x57, 0xbb]);
    }

    #[tokio::test]
    async fn write_read() {
        let mut v = Vec::new();
        let h = P2PMessageHeader {
            magic: [0x00, 0x01, 0x02, 0x03],
            command: *b"command\0\0\0\0\0",
            payload_size: 42,
            checksum: [0xa0, 0xa1, 0xa2, 0xa3],
        };
        h.write(&mut v).await.unwrap();
        assert_eq!(v.len(), h.size());
        assert_eq!(P2PMessageHeader::read(&mut Cursor::new(&v)).await.unwrap(), h);
    }

    #[test]
    fn validate() {
        let magic = [0xa0, 0xa1, 0xa2, 0xa3];
        let h = P2PMessageHeader {
            magic,
            command: *b"verack\0\0\0\0\0\0",
            payload_size: 88,
            checksum: [0x12, 0x34, 0x56, 0x78],
        };
        // Valid
        assert!(h.validate(magic, 100).is_ok());
        // Bad magic
        let bad_magic = [0xb0, 0xb1, 0xb2, 0xb3];
        assert!(h.validate(bad_magic, 100).is_err());
        // Bad size
        assert!(h.validate(magic, 50).is_err());
    }
}
