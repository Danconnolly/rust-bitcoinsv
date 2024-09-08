use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead};
use tokio::sync::mpsc;
use tokio_stream::Stream;
use crate::bitcoin::{BlockHeader, AsyncEncodable, Tx, varint_decode};
use crate::BsvResult;


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
///
/// If the struct is dropped, the background task will terminate, although this may not be immediate.
// Notes on background task termination:
// the tokio JoinHandle documentation (https://docs.rs/tokio/1.37.0/tokio/task/struct.JoinHandle.html)
// is specific: "If a JoinHandle is dropped, then the task continues running in the background and
// its return value is lost."
// This means that the background task could continue to run even if the FullBlockStream is dropped,
// which is not what we want.
// However, in our case the background task is continually sending to a channel. When the FullBlockStream
// struct is dropped, the receiver of the channel will be dropped. The sender (in the background task)
// will then return an error when it tries to send to the channel. This will cause the background task
// to terminate.
// I don't have any real tests for this, very keen to hear any ideas.
pub struct FullBlockStream {
    /// The block header for this block.
    pub block_header: BlockHeader,
    /// The number of transactions in this block.
    pub num_tx: u64,
    // the channel receiver for transactions
    receiver: mpsc::Receiver<BsvResult<Tx>>,
}

// The size of the transaction buffer, this buffer is filled by a background task.
const BUFFER_SIZE: usize = 1_000;

impl FullBlockStream {
    /// Create a new FullBlockStream, decoding the block from the reader. The block header and the
    /// number of transactions in the block will be immediately ready when this function has finished.
    /// The buffer size is set to 1_000.
    pub async fn new(reader: Box<dyn AsyncRead + Unpin + Send>) -> BsvResult<FullBlockStream> {
        FullBlockStream::new_bufsize(reader, BUFFER_SIZE).await
    }

    /// Create a new FullBlockStream, decoding the block from the reader, with the buffer size
    /// specified. The block header and the number of transactions in the block will be immediately
    /// ready when this function has finished.
    pub async fn new_bufsize(mut reader: Box<dyn AsyncRead + Unpin + Send>, buf_size: usize) -> BsvResult<FullBlockStream> {
        // read block header and number of transactions
        let block_header = BlockHeader::from_binary(&mut reader).await?;
        let num_tx = varint_decode(&mut reader).await?;
        let (sender, rx) = mpsc::channel::<BsvResult<Tx>>(buf_size);
        // spawn a task to continuously read transactions from the reader and send them to the channel
        let mut tx_reader = FullBlockTxReader::new(num_tx, reader, sender);
        let _h = tokio::spawn(async move {tx_reader.read_tx().await});
        Ok(FullBlockStream {
            block_header, num_tx, receiver: rx,
        })
    }
}

impl Stream for FullBlockStream {
    type Item = BsvResult<Tx>;

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
    sender: mpsc::Sender<BsvResult<Tx>>,
}

impl FullBlockTxReader {
    pub fn new(num_tx: u64, reader: Box<dyn AsyncRead + Unpin + Send>, sender: mpsc::Sender<BsvResult<Tx>>) -> Self {
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
    // Use a small buffer size and pause for a short time before the first transaction read.
    // this will cause the background task to also pause, and we can check that it resumes and is
    // not terminated.
    #[tokio::test]
    async fn test_full_block_stream() {
        let block_bin = get_small_block_bin().await;
        let cursor = Box::new(Cursor::new(block_bin));
        let mut s = FullBlockStream::new_bufsize(cursor, 1).await.unwrap();
        assert_eq!(s.block_header.hash(), Hash::from_hex("0000000000000000000988036522057056727ae85ad7cea92b2198418c9bb8f7").unwrap());
        assert_eq!(s.num_tx, 222);
        // pause for a short time before reading the first transaction
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
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
