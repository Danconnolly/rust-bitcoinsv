use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use crate::bitcoin::{BlockHeader, AsyncEncodable, Tx, varint_decode, varint_encode, varint_size};

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
impl AsyncEncodable for Block {
    async fn async_from_binary<R: AsyncRead + Unpin + Send>(reader: &mut R) -> crate::BsvResult<Self> where Self: Sized {
        let header = BlockHeader::async_from_binary(reader).await?;
        let txn_count = varint_decode(reader).await? as usize;
        // todo: check for too many transactions
        let mut transactions = Vec::with_capacity(txn_count);
        for _ in 0..txn_count {
            transactions.push(Tx::async_from_binary(reader).await?);
        }
        Ok(Block { header, transactions })
    }

    async fn async_to_binary<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> crate::BsvResult<()> {
        self.header.async_to_binary(writer).await?;
        varint_encode(writer, self.transactions.len() as u64).await?;
        for txn in self.transactions.iter() {
            txn.async_to_binary(writer).await?;
        }
        Ok(())
    }

    fn async_size(&self) -> usize {
        self.header.async_size() + varint_size(self.transactions.len() as u64) + self.transactions.iter().map(|t| t.async_size()).sum::<usize>()
    }
}
