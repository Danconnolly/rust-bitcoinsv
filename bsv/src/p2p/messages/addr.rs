use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use crate::bitcoin::{Encodable, varint_decode, varint_encode, varint_size};
use crate::p2p::messages::NodeAddr;

/// Addr message. This message is sent to advertise known nodes to the network.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Addr {
    /// List of addresses of known nodes
    pub addrs: Vec<NodeAddr>,
}

impl Addr {
    /// Maximum number of addresses allowed in an Addr message
    pub const MAX_ADDR_COUNT: u64 = 1000;
}

#[async_trait]
impl Encodable for Addr {
    async fn decode_from<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self> where Self: Sized {
        let i = varint_decode(reader).await?;
        if i > Addr::MAX_ADDR_COUNT {
            let msg = format!("Too many addrs: {}", i);
            return Err(crate::Error::BadData(msg));
        }
        let mut addrs = Vec::with_capacity(i as usize);
        for _ in 0..i {
            addrs.push(NodeAddr::decode_from(reader).await?);
        }
        Ok(Addr { addrs })
    }

    async fn encode_into<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()> {
        if self.addrs.len() as u64 > Addr::MAX_ADDR_COUNT {
            let msg = format!("Too many addrs: {}", self.addrs.len());
            return Err(crate::Error::BadData(msg));
        }
        varint_encode(writer, self.addrs.len() as u64).await?;
        for addr in self.addrs.iter() {
            addr.encode_into(writer).await?;
        }
        Ok(())
    }

    fn size(&self) -> usize {
        varint_size(self.addrs.len() as u64) + self.addrs.len() * NodeAddr::SIZE
    }
}
