use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
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

#[async_trait]
impl Encodable for Ping {
    async fn decode_from<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self> where Self: Sized {
        let nonce = reader.read_u64_le().await?;
        Ok(Ping { nonce })
    }

    async fn encode_into<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()> {
        writer.write_u64_le(self.nonce).await?;
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

    #[test]
    fn read_bytes() {
        let b = hex::decode("86b19332b96c657d".as_bytes()).unwrap();
        let f = Ping::decode_from_buf(b.as_slice()).unwrap();
        assert_eq!(f.nonce, 9035747770062057862);
    }

    #[test]
    fn write_read() {
        let p = Ping { nonce: 13579 };
        let v = p.encode_into_buf().unwrap();
        assert_eq!(v.len(), p.size());
        assert_eq!(Ping::decode_from_buf(v.as_slice()).unwrap(), p);
    }
}
