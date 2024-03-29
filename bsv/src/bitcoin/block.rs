use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::Stream;
use crate::bitcoin::{BlockHeader, Encodable, Tx, varint_decode};
use crate::Result;


/// Deserialize the bytes in a block and produce a stream of the transactions.
///
/// It also provides access to the block header.
///
/// In Bitcoin SV, blocks can get very large, so we dont actually keep a block in memory, but
/// instead iterate over the transactions in the block. Ideally each transaction should be processed
/// immediately and dropped to keep memory use low.
///
/// Create the FullBlockStream using a reader. Once initialized, the FullBlockStream will make
/// the BlockHeader and the number of transactions available. You can then iterate over the
/// transactions using the tokio_stream::StreamExt trait.
///
/// Something like:
///         let mut steam = FullBlockStream::new(reader).await.unwrap();
///         let hdr = stream.block_header;
///         let num_tx = stream.num_tx;
///         while let Some(tx) = s.next().await {
///             // process tx
///         }
///
/// Transactions are read using a background task and queued in a channel, ready for delivery to
/// the calling task.
pub struct FullBlockStream {
    /// The block header for this block.
    pub block_header: BlockHeader,
    /// The number of transactions in this block.
    pub num_tx: u64,
    // the channel receiver for transactions
    receiver: mpsc::Receiver<Result<Tx>>,
    // the background task that reads transactions from the block
    // although we dont use this, we need to keep it in scope to keep the task running
    _bgrnd_task: JoinHandle<()>,
}

// The size of the transaction buffer, this buffer is filled by a background task.
const BUFFER_SIZE: usize = 1000;

impl FullBlockStream {
    /// Create a new FullBlockStream, decoding the block from the reader. The block header and the
    /// number of transactions in the block will be immediately ready when this function has finished.
    pub async fn new(mut reader: Box<dyn AsyncRead + Unpin + Send>) -> crate::Result<FullBlockStream> {
        // read block header and number of transactions
        let block_header = BlockHeader::from_binary(&mut reader).await?;
        let num_tx = varint_decode(&mut reader).await?;
        let (sender, rx) = mpsc::channel::<crate::Result<Tx>>(BUFFER_SIZE);
        // spawn a task to continuously read transactions from the reader and send them to the channel
        let mut tx_reader = FullBlockTxReader::new(num_tx, reader, sender);
        let h = tokio::spawn(async move {tx_reader.read_tx().await});
        Ok(FullBlockStream {
            block_header, num_tx, receiver: rx, _bgrnd_task: h,
        })
    }
}

impl Stream for FullBlockStream {
    type Item = crate::Result<Tx>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // return the next item from the channel
        Pin::new(&mut self.receiver).poll_recv(cx)
    }
}

/// The background task that reads transactions from the reader and sends them to the channel for
/// the FullBlockStream.
struct FullBlockTxReader {
    num_tx: u64,
    reader: Box<dyn AsyncRead + Unpin + Send>,
    sender: mpsc::Sender<Result<Tx>>,
}

impl FullBlockTxReader {
    pub fn new(num_tx: u64, reader: Box<dyn AsyncRead + Unpin + Send>, sender: mpsc::Sender<Result<Tx>>) -> Self {
        FullBlockTxReader {
            num_tx, reader, sender,
        }
    }

    async fn read_tx(&mut self) {
        for _ in 0..self.num_tx {
            let t = Tx::from_binary(&mut self.reader).await;
            match t {
                Ok(tx) => {
                    if self.sender.send(Ok(tx)).await.is_err() {
                        break; // Receiver has dropped
                    }
                }
                Err(e) => {
                    let _ = self.sender.send(Err(e)).await; // Send error and break
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use hex::FromHex;
    use tokio::fs::File;
    use super::*;
    use tokio::io::AsyncReadExt;
    use tokio_stream::StreamExt;
    use crate::bitcoin::hash::Hash;

    // Stream a block and check that we can read the transactions from it.
    #[tokio::test]
    async fn test_full_block_stream() {
        let block_bin = get_small_block_bin().await;
        let cursor = Box::new(Cursor::new(block_bin));
        let mut s = FullBlockStream::new(cursor).await.unwrap();
        assert_eq!(s.block_header.hash(), Hash::from_hex("0000000000000000000988036522057056727ae85ad7cea92b2198418c9bb8f7").unwrap());
        assert_eq!(s.num_tx, 222);
        let mut tx_count = 0;
        while let Some(tx) = s.next().await {
            tx_count += 1;
            tx.unwrap();
        }
        assert_eq!(tx_count, 222);
    }

    // read block from a file for test purposes
    async fn get_small_block_bin() -> Vec<u8> {
        let mut file = File::open("../testdata/0000000000000000000988036522057056727ae85ad7cea92b2198418c9bb8f7.bin").await.expect("Could not open file");
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await.expect("Could not read file");
        buffer
    }
}
