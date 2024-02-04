use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use crate::bitcoin::{BlockHeader, Encodable, Tx, varint_decode, varint_encode, varint_size};

/// A Block message is sent in response to a `getdata` message. It contains the header and every
/// transaction in the block.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Block {
    /// The block header
    pub header: BlockHeader,
    /// The transactions in the block
    pub transactions: Vec<Tx>,
}

#[async_trait]
impl Encodable for Block {
    async fn decode_from<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::Result<Self> where Self: Sized {
        let header = BlockHeader::decode_from(reader).await?;
        let txn_count = varint_decode(reader).await? as usize;
        // todo: check for too many transactions
        let mut transactions = Vec::with_capacity(txn_count);
        for _ in 0..txn_count {
            transactions.push(Tx::decode_from(reader).await?);
        }
        Ok(Block { header, transactions })
    }

    async fn encode_into<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::Result<()> {
        self.header.encode_into(writer).await?;
        varint_encode(writer, self.transactions.len() as u64).await?;
        for txn in self.transactions.iter() {
            txn.encode_into(writer).await?;
        }
        Ok(())
    }

    fn size(&self) -> usize {
        self.header.size() + varint_size(self.transactions.len() as u64) + self.transactions.iter().map(|t| t.size()).sum::<usize>()
    }
}
