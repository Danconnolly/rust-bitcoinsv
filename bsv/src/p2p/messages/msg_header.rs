use crate::{BsvError, BsvResult};
use std::fmt;
use std::str;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::bitcoin::AsyncEncodable;
use crate::p2p::messages::messages::commands::{BLOCK, EXTMSG};
use crate::p2p::messages::messages::PROTOCONF;
use crate::p2p::messages::protoconf::MAX_PROTOCONF_SIZE;
use crate::p2p::stream::StreamConfig;

// based on code imported from rust-sv but substantially modified

/// Header that begins all messages.
///
/// We have collapsed the standard header and the extended header into one struct.
/// The extended header is described in [P2P Large Message Support](https://github.com/bitcoin-sv-specs/protocol/blob/master/p2p/large_messages.md).
#[derive(Default, PartialEq, Eq, Hash, Clone)]
pub struct P2PMessageHeader {
    /// Magic bytes indicating the network type
    pub magic: [u8; 4],
    /// Command name
    pub command: [u8; 12],
    /// Payload size
    pub payload_size: u64,
    /// First 4 bytes of SHA256(SHA256(payload))
    pub checksum: [u8; 4],
}

impl P2PMessageHeader {
    /// Size of the standard message header in bytes
    pub const STANDARD_SIZE: usize = 24;
    /// Size of the extended message header in bytes
    pub const EXTENDED_SIZE: usize = 44;

    /// Returns true if the header is in extended format.
    pub fn is_extended(&self) -> bool {
        self.payload_size >= 0xffffffff && self.command == BLOCK
    }

    /// Checks if the header is valid
    ///
    /// `magic` - Expected magic bytes for the network
    /// `max_size` - Max size in bytes for the payload
    pub fn validate(&self, config: &StreamConfig) -> BsvResult<()> {
        if self.magic != config.magic {
            // todo: ban
            let msg = format!("Bad magic: {:02x},{:02x},{:02x},{:02x}", self.magic[0], self.magic[1], self.magic[2], self.magic[3]);
            return Err(BsvError::BadData(msg));
        }
        if self.command == PROTOCONF {
            // strange exception for protoconf messages
            return if self.payload_size > MAX_PROTOCONF_SIZE {
                // todo: ban score
                let msg = format!("Bad size for protoconf message: {:?}", self.payload_size);
                Err(BsvError::BadData(msg))
            } else {
                Ok(())
            }
        }
        if self.command == BLOCK {       // normal payload size limit does not apply to block messages
            return if self.payload_size > config.excessive_block_size {
                // todo: ban score
                let msg = format!("Bad size for block message: {:?}", self.payload_size);
                Err(BsvError::BadData(msg))
            } else {
                Ok(())
            }
        }
        if self.payload_size > config.max_recv_payload_size {
            // todo: ban score
            let msg = format!("Bad size: {:?}", self.payload_size);
            return Err(BsvError::BadData(msg));
        }
        Ok(())
    }
}

#[async_trait]
impl AsyncEncodable for P2PMessageHeader {
    async fn async_from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> BsvResult<Self> where Self: Sized {
        // read standard header
        let mut magic = vec![0u8; 4];
        reader.read_exact(&mut magic).await?;
        let mut command = vec![0u8; 12];
        reader.read_exact(&mut command).await?;
        let mut payload_size: u64 = reader.read_u32_le().await? as u64;
        let mut checksum = vec![0u8; 4];
        reader.read_exact(&mut checksum).await?;
        if command == EXTMSG {
            // its an extended header
            reader.read_exact(&mut command).await?;     // re-read the command
            payload_size = reader.read_u64_le().await?;
            if payload_size < 0xffffffff {
                return Err(BsvError::BadData("used extended header for small payload".to_string()));
            }
            if command != BLOCK {
                return Err(BsvError::BadData("unknown command in extended header".to_string()));
            }
        }
        Ok(P2PMessageHeader { magic: magic.try_into().unwrap(), command: command.try_into().unwrap(),
            payload_size, checksum: checksum.try_into().unwrap(), })
    }

    async fn async_to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> BsvResult<()> {
        // do we need to write an extended header?
        if self.is_extended() {
            writer.write_all(&self.magic).await?;
            writer.write_all(&EXTMSG).await?;
            writer.write_u32_le(0xffffffff).await?;
            writer.write_all(&self.checksum).await?;
            writer.write_all(&self.command).await?;
            writer.write_u64_le(self.payload_size).await?;
            Ok(())
        } else {
            writer.write_all(&self.magic).await?;
            writer.write_all(&self.command).await?;
            writer.write_u32_le(self.payload_size as u32).await?;
            writer.write_all(&self.checksum).await?;
            Ok(())
        }
    }

    fn async_size(&self) -> usize {
        if self.is_extended() {
            P2PMessageHeader::EXTENDED_SIZE
        } else {
            P2PMessageHeader::STANDARD_SIZE
        }
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

    #[test]
    fn read_bytes() {
        let b = hex::decode("f9beb4d976657273696f6e00000000007a0000002a1957bb".as_bytes()).unwrap();
        let h = P2PMessageHeader::from_binary_buf(b.as_slice()).unwrap();
        assert_eq!(h.magic, [0xf9, 0xbe, 0xb4, 0xd9]);
        assert_eq!(h.command, *b"version\0\0\0\0\0");
        assert_eq!(h.payload_size, 122);
        assert_eq!(h.checksum, [0x2a, 0x19, 0x57, 0xbb]);
    }

    #[test]
    fn write_read() {
        let h = P2PMessageHeader {
            magic: [0x00, 0x01, 0x02, 0x03],
            command: *b"command\0\0\0\0\0",
            payload_size: 42,
            checksum: [0xa0, 0xa1, 0xa2, 0xa3],
        };
        let v = h.to_binary_buf().unwrap();
        assert_eq!(v.len(), h.async_size());
        assert_eq!(P2PMessageHeader::from_binary_buf(v.as_slice()).unwrap(), h);
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
        let mut config = StreamConfig::default();
        config.magic = magic.clone();
        // Valid
        assert!(h.validate(&config).is_ok());
        // Bad magic
        let bad_magic = [0xb0, 0xb1, 0xb2, 0xb3];
        let mut bad_config = config.clone();
        bad_config.magic = bad_magic;
        assert!(h.validate(&bad_config).is_err());
        // Bad size
        assert!(h.validate(&bad_config).is_err());
    }
}
