use crate::bitcoin::{varint_decode_async, varint_encode_async, AsyncEncodable, BlockHeader};
use async_trait::async_trait;
use std::fmt;
use tokio::io::{AsyncRead, AsyncWrite};

/// List of block headers
#[derive(Default, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Headers {
    /// List of sequential block headers
    pub headers: Vec<BlockHeader>,
}

#[cfg(feature = "dev_tokio")]
#[async_trait]
impl AsyncEncodable for Headers {
    async fn async_from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let num_headers = varint_decode_async(reader).await? as usize;
        let mut headers = Vec::with_capacity(num_headers);
        for _ in 0..num_headers {
            headers.push(BlockHeader::async_from_binary(reader).await?);
        }
        Ok(Headers { headers })
    }

    async fn async_to_binary<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> crate::Result<()> {
        varint_encode_async(writer, self.headers.len() as u64).await?;
        for header in self.headers.iter() {
            header.async_to_binary(writer).await?;
        }
        Ok(())
    }

    // todo: add Encodable trait
    // fn async_size(&self) -> usize {
    //     varint_size(self.headers.len() as u64) + self.headers.len() * BlockHeader::SIZE
    // }
}

impl fmt::Display for Headers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut out_str = String::new();
        for header in &self.headers {
            if out_str.is_empty() {
                out_str = format!("{:?}", header);
            } else {
                out_str += &*format!(", {:?}", header);
            }
        }
        write!(f, "Headers(n={}, [{}])", self.headers.len(), out_str)
    }
}
