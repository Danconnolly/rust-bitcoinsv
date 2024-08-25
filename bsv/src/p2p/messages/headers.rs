use std::fmt;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use crate::bitcoin::{BlockHeader, Encodable, varint_decode, varint_encode, varint_size};


/// List of block headers
#[derive(Default, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Headers {
    /// List of sequential block headers
    pub headers: Vec<BlockHeader>,
}

#[async_trait]
impl Encodable for Headers {
    async fn from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::BsvResult<Self> where Self: Sized {
        let num_headers = varint_decode(reader).await? as usize;
        let mut headers = Vec::with_capacity(num_headers);
        for _ in 0..num_headers {
            headers.push(BlockHeader::from_binary(reader).await?);
        }
        Ok(Headers { headers })
    }

    async fn to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::BsvResult<()> {
        varint_encode(writer, self.headers.len() as u64).await?;
        for header in self.headers.iter() {
            header.to_binary(writer).await?;
        }
        Ok(())
    }

    fn size(&self) -> usize {
        varint_size(self.headers.len() as u64) + self.headers.len() * BlockHeader::SIZE
    }
}

impl fmt::Display for Headers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut out_str = String::new();
        for header in &self.headers {
            if out_str.len() == 0 {
                out_str = format!("{:?}", header);
            } else {
                out_str += &*format!(", {:?}", header);
            }
        }
        write!(f, "Headers(n={}, [{}])", self.headers.len(), out_str)
    }
}
