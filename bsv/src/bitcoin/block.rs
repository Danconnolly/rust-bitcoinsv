use tokio::io::AsyncRead;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
pub use tokio_stream::StreamExt;
use crate::bitcoin::{BlockHeader, Tx};


/// Deserialize the bytes in a block and produce a stream of the transactions.
///
/// It also provides access to the block header.
///
/// In Bitcoin SV, blocks can get very large, so we dont actually keep a block in memory, but
/// instead iterate over the transactions in the block. Ideally each transaction should be processed
/// immediately and dropped to keep memory use low.
/// In this library we dont provide any other mechanism for accessing the transactions in a block,
/// in order to avoid the temptation to keep the block in memory.
///
/// Create the FullBlockStream using a reader. Once initialized, the FullBlockStream will make
/// the BlockHeader and the number of transactions available. You can then iterate over the
/// transactions using the tokio_stream::StreamExt trait. At the end you can retrieve the reader
/// from the FullBlockStream using the finish() method but note that it will need to have
/// read all of the transactions in the block before it can be retrieved.
///
/// Something like:
///        let mut steam = FullBlockStream::new(reader).await.unwrap();
///         while let Some(tx) = s.next().await {
///             // process tx
///         }
///         let r = s.finish().await;
///
/// Transactions are read using a background task and queued in a channel, ready for delivery to
/// the calling task.
pub struct FullBlockStream<R>
    where
        R: AsyncRead + Unpin + Send,
{
    pub block_header: BlockHeader,
    pub num_tx: u64,
    receiver: mpsc::Receiver<crate::Result<Tx>>,
    reader_handle: Option<JoinHandle<R>>,
}

// this is the size of the buffer in the channel.
const BUFFER_SIZE: usize = 1000;

// impl<R> FullBlockStream<R>       todo
//     where
//         R: AsyncRead + Unpin + Send + 'static,
// {
//     /// Create a new FullBlockStream, decoding the block from the reader. The block header and the
//     /// number of transactions in the block will be immediately ready when this function has finished.
//     pub async fn new(mut reader: R) -> crate::Result<FullBlockStream<R>> {
//         let block_header = BlockHeader::read(&mut reader).await?;
//         let num_tx = VarInt::read(&mut reader).await?.value;
//         let (sender, rx) = mpsc::channel::<crate::Result<Tx>>(BUFFER_SIZE);
//         // spawn a task to continuously read transactions from the reader and send them to the channel
//         let mut tx_reader = FullBlockTxReader::new(num_tx, reader, sender);
//         let h = tokio::spawn(async move {tx_reader.read_tx().await});
//         Ok(FullBlockStream {
//             block_header, num_tx, receiver: rx, reader_handle: Some(h),
//         })
//     }
//
//     /// Retrieve the reader from the FullBlockStream. This will block until all transactions have
//     /// been read. Note that the FullBlockStream contains an internal buffer of limited size and if
//     /// this buffer fills up and is not cleared, the reader will block forever.
//     pub async fn finish(&mut self) -> R {
//         let h = self.reader_handle.take().unwrap();
//         h.await.unwrap()
//     }
// }
//
// impl<R> Stream for FullBlockStream<R>
//     where
//         R: AsyncRead + Unpin + Send,
// {
//     type Item = crate::Result<Tx>;
//     fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         Pin::new(&mut self.receiver).poll_recv(cx)
//     }
// }

/// The background task that reads transactions from the reader and sends them to the channel for
/// the FullBlockStream.
struct FullBlockTxReader<R>
    where
        R: AsyncRead + Unpin + Send,
{
    num_tx: u64,
    reader: Option<R>,
    sender: mpsc::Sender<crate::Result<Tx>>,
}

// impl<R> FullBlockTxReader<R>
//     where
//         R: AsyncRead + Unpin + Send,
// {
//     pub fn new(num_tx: u64, reader: R, sender: mpsc::Sender<crate::Result<Tx>>) -> FullBlockTxReader<R> {
//         FullBlockTxReader {
//             num_tx, reader: Some(reader), sender,
//         }
//     }
//
//     async fn read_tx(&mut self) -> R {
//         let mut r = self.reader.take().unwrap();
//         for _ in 0..self.num_tx {
//             match Tx::read(&mut r).await {
//                 Ok(tx) => {
//                     if self.sender.send(Ok(tx)).await.is_err() {
//                         break; // Receiver has dropped
//                     }
//                 }
//                 Err(e) => {
//                     let _ = self.sender.send(Err(e)).await; // Send error and break
//                     break;
//                 }
//             }
//         }
//         r
//     }
// }

// #[cfg(test)]
// mod tests {
//     use std::io::Cursor;
//     use hex::FromHex;
//     use tokio::fs::File;
//     use super::*;
//     use tokio::io::AsyncReadExt;
//     use tokio_stream::StreamExt;
//     use crate::bitcoin::hash::Hash;
//
//     // Stream a block and check that we can read the transactions from it.
//     #[tokio::test]
//     async fn test_full_block_stream() {
//         let block_bin = get_small_block_bin().await;
//         let cursor = Cursor::new(block_bin);
//         let mut s = FullBlockStream::new(cursor).await.unwrap();
//         assert_eq!(s.block_header.hash(), Hash::from_hex("0000000000000000000988036522057056727ae85ad7cea92b2198418c9bb8f7").unwrap());
//         assert_eq!(s.num_tx, 222);
//         let mut tx_count = 0;
//         while let Some(tx) = s.next().await {
//             tx_count += 1;
//             tx.unwrap();
//         }
//         assert_eq!(tx_count, 222);
//         let r = s.finish().await;
//         assert_eq!(r.position(), r.get_ref().len() as u64, "Cursor is not at the end");
//     }
//
//     // read block from a file for test purposes
//     async fn get_small_block_bin() -> Vec<u8> {
//         let mut file = File::open("../testdata/0000000000000000000988036522057056727ae85ad7cea92b2198418c9bb8f7.bin").await.expect("Could not open file");
//         let mut buffer = Vec::new();
//         file.read_to_end(&mut buffer).await.expect("Could not read file");
//         buffer
//     }
// }
