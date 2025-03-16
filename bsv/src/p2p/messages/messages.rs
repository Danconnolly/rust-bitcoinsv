pub use self::commands::PROTOCONF;
use crate::bitcoin::{AsyncEncodable, Hash, Tx};
use crate::p2p::channel::ChannelConfig;
use crate::p2p::messages::addr::Addr;
use crate::p2p::messages::block::Block;
use crate::p2p::messages::block_locator::BlockLocator;
use crate::p2p::messages::headers::Headers;
use crate::p2p::messages::inv::Inv;
use crate::p2p::messages::merkle_block::MerkleBlock;
use crate::p2p::messages::messages::commands::{
    ADDR, BLOCK, GETADDR, GETBLOCKS, GETDATA, GETHEADERS, HEADERS, INV, MEMPOOL, MERKLEBLOCK,
    NOTFOUND, PING, PONG, REJECT, SENDCMPCT, SENDHEADERS, TX, VERACK, VERSION,
};
use crate::p2p::messages::messages::P2PMessageType::{ConnectionControl, Data};
use crate::p2p::messages::msg_header::P2PMessageHeader;
use crate::p2p::messages::protoconf::Protoconf;
use crate::p2p::messages::reject::Reject;
use crate::p2p::messages::send_cmpct::SendCmpct;
use crate::p2p::messages::{Ping, Version};
use crate::{Error, Result};
use log::{trace, warn};
use std::fmt;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// based on code imported from rust-sv but substantially modified

// I wont be implementing the FEEFILTER related messages. These aren't scalable. As unknown messages,
// they will be ignored if received.
//
// The Bitcoin P2P protocol is a message based protocol, where each message is a chunk of contiguous data.
// At first glance, the messages appear to represent a single fact or a collection of independent facts. However, on
// closer examination most of the messages that contain lists of items attribute significance to the list itself and the
// position of items within the list.
//      ADDR - this is an exception, there's no significance to the ordering of the list and the items are independent
//      INV (tx) - transactions are supposed to be in this list in order of dependence, with parent transactions
//                  preceding children. This cannot be relied upon though.
//      GETDATA - like the inv, the order of transactions in this list is significant.
//      NOTFOUND - ordering not significant, items independent
//      GETBLOCKS - ordering specifically relevant
//      GETHEADERS - ordering specifically relevant
//      HEADERS - ordering specifically relevant
// Given the significance of ordering within these messages, there is no benefit from breaking the messages up
// into smaller parts and streaming those parts.
//
// Most of the messages are also reasonably small, the messages are quickly transferred between
// peers. There is a maximum message size which limits the amount of memory that will be allocated to deal
// with a message in its entirety.
//
// One exception is the BLOCK message, which transfers a block in its entirety. With large blocks
// (up to 4GB at the time of writing), this can cause a significant memory issue and we will eventually have special
// handling for this message. But its not just the memory, transferring a 4GB block takes significant amounts of time,
// and we want our code to start processing the block as soon as possible, otherwise it adds a significant delay to the
// block processing time. So we definitely want to use a streaming model for this. <IMPROVEMENT - do this>
//
// The TX message is a potential concern. Transactions can get large but there is also a policy on the SV Node
// regarding the maximum size of transactions.  <IMPROVEMENT - check this and adjust>
//
// Each message consists of a header and a payload. The header starts with a set of "magic" bytes that identify both
// the blockchain to which these messages apply (mainnet, testnet, regtest, stn). The header also contains the size of
// the payload. Our code needs to protect against cases where the payload size specified in the header is incorrect.

// Given the above, we will be implementing an asynchronous reading model for the BLOCK message. To maintian consistency
// we will also be implementing the same model for all messages.

/// Checksum to use when there is an empty payload.
pub const NO_CHECKSUM: [u8; 4] = [0x5d, 0xf6, 0xe0, 0xe2];
/// Checksum to use when using extended message header
pub const ZERO_CHECKSUM: [u8; 4] = [0, 0, 0, 0];

/// Message commands for the header
pub mod commands {
    /// [Addr command](https://en.bitcoin.it/wiki/Protocol_documentation#addr)
    pub const ADDR: [u8; 12] = *b"addr\0\0\0\0\0\0\0\0";

    /// [Alert command](https://en.bitcoin.it/wiki/Protocol_documentation#alert) (deprecated)
    pub const ALERT: [u8; 12] = *b"alert\0\0\0\0\0\0\0";

    /// [Block command](https://en.bitcoin.it/wiki/Protocol_documentation#block)
    pub const BLOCK: [u8; 12] = *b"block\0\0\0\0\0\0\0";

    /// [Block transaction command](https://en.bitcoin.it/wiki/Protocol_documentation#blocktxn)
    pub const BLOCKTXN: [u8; 12] = *b"blocktxn\0\0\0\0";

    /// [Compact block command](https://en.bitcoin.it/wiki/Protocol_documentation#cmpctblock)
    pub const CMPCTBLOCK: [u8; 12] = *b"cmpctblock\0\0";

    /// [Extended Message Header](https://github.com/bitcoin-sv-specs/protocol/blob/master/p2p/large_messages.md)
    pub const EXTMSG: [u8; 12] = *b"extmsg\0\0\0\0\0\0";

    /// [Inventory command](https://en.bitcoin.it/wiki/Protocol_documentation#inv)
    pub const INV: [u8; 12] = *b"inv\0\0\0\0\0\0\0\0\0";

    /// [Get addr command](https://en.bitcoin.it/wiki/Protocol_documentation#getaddr)
    pub const GETADDR: [u8; 12] = *b"getaddr\0\0\0\0\0";

    /// [Get blocks command](https://en.bitcoin.it/wiki/Protocol_documentation#getblocks)
    pub const GETBLOCKS: [u8; 12] = *b"getblocks\0\0\0";

    /// [Get block transaction command](https://en.bitcoin.it/wiki/Protocol_documentation#getblocktxn)
    pub const GETBLOCKTXN: [u8; 12] = *b"getblocktxn\0";

    /// [Get data command](https://en.bitcoin.it/wiki/Protocol_documentation#getdata)
    pub const GETDATA: [u8; 12] = *b"getdata\0\0\0\0\0";

    /// [Get headers command](https://en.bitcoin.it/wiki/Protocol_documentation#getheaders)
    pub const GETHEADERS: [u8; 12] = *b"getheaders\0\0";

    /// [Headers command](https://en.bitcoin.it/wiki/Protocol_documentation#headers)
    pub const HEADERS: [u8; 12] = *b"headers\0\0\0\0\0";

    /// [Mempool command](https://en.bitcoin.it/wiki/Protocol_documentation#mempool)
    pub const MEMPOOL: [u8; 12] = *b"mempool\0\0\0\0\0";

    /// [Merkle block](https://en.bitcoin.it/wiki/Protocol_documentation#filterload.2C_filteradd.2C_filterclear.2C_merkleblock)
    pub const MERKLEBLOCK: [u8; 12] = *b"merkleblock\0";

    /// [Not found command](https://en.bitcoin.it/wiki/Protocol_documentation#notfound)
    pub const NOTFOUND: [u8; 12] = *b"notfound\0\0\0\0";

    /// [Ping command](https://en.bitcoin.it/wiki/Protocol_documentation#ping)
    pub const PING: [u8; 12] = *b"ping\0\0\0\0\0\0\0\0";

    /// [Pong command](https://en.bitcoin.it/wiki/Protocol_documentation#pong)
    pub const PONG: [u8; 12] = *b"pong\0\0\0\0\0\0\0\0";

    /// [Protoconf command](https://github.com/bitcoin-sv-specs/protocol/blob/master/p2p/protoconf.md)
    pub const PROTOCONF: [u8; 12] = *b"protoconf\0\0\0";

    /// [Reject command](https://en.bitcoin.it/wiki/Protocol_documentation#reject)
    pub const REJECT: [u8; 12] = *b"reject\0\0\0\0\0\0";

    /// [Send compact command](https://en.bitcoin.it/wiki/Protocol_documentation#sendcmpct)
    pub const SENDCMPCT: [u8; 12] = *b"sendcmpct\0\0\0";

    /// [Send headers command](https://en.bitcoin.it/wiki/Protocol_documentation#sendheaders)
    pub const SENDHEADERS: [u8; 12] = *b"sendheaders\0";

    /// [Transaction command](https://en.bitcoin.it/wiki/Protocol_documentation#tx)
    pub const TX: [u8; 12] = *b"tx\0\0\0\0\0\0\0\0\0\0";

    /// [Version command](https://en.bitcoin.it/wiki/Protocol_documentation#version)
    pub const VERSION: [u8; 12] = *b"version\0\0\0\0\0";

    /// [Version acknowledgement command](https://en.bitcoin.it/wiki/Protocol_documentation#verack)
    pub const VERACK: [u8; 12] = *b"verack\0\0\0\0\0\0";
}

/// Bitcoin peer-to-peer message with its payload
#[derive(PartialEq, Eq, Hash, Clone)]
pub enum P2PMessage {
    Addr(Addr),
    Block(Block),
    GetAddr,
    GetBlocks(BlockLocator),
    GetData(Inv),
    GetHeaders(BlockLocator),
    Headers(Headers),
    Inv(Inv),
    Mempool,
    MerkleBlock(MerkleBlock),
    NotFound(Inv),
    Ping(Ping),
    Pong(Ping),
    Protoconf(Protoconf),
    Reject(Reject),
    SendHeaders,
    SendCmpct(SendCmpct),
    Tx(Tx),
    Verack,
    Version(Version),
    Unknown(String, usize),
}

impl P2PMessage {
    // /// Read a full P2P message from the reader
    // pub async fn read<R: AsyncRead + Unpin + Send>(
    //     reader: &mut R,
    //     comms_config: &ChannelConfig,
    // ) -> Result<Self> {
    //     let header = P2PMessageHeader::async_from_binary(reader).await?;
    //     trace!("P2PMessage::read() - header: {:?}", header);
    //     header.validate(comms_config)?;
    //     // payload size has been checked for max limit in header.validate()
    //     let msg = match header.command {
    //         ADDR => P2PMessage::Addr(Addr::async_from_binary(reader).await?),
    //         BLOCK => P2PMessage::Block(Block::async_from_binary(reader).await?),
    //         GETADDR => P2PMessage::GetAddr,
    //         GETBLOCKS => P2PMessage::GetBlocks(BlockLocator::async_from_binary(reader).await?),
    //         GETDATA => P2PMessage::GetData(Inv::async_from_binary(reader).await?),
    //         GETHEADERS => P2PMessage::GetHeaders(BlockLocator::async_from_binary(reader).await?),
    //         HEADERS => P2PMessage::Headers(Headers::async_from_binary(reader).await?),
    //         INV => P2PMessage::Inv(Inv::async_from_binary(reader).await?),
    //         MEMPOOL => P2PMessage::Mempool,
    //         MERKLEBLOCK => P2PMessage::MerkleBlock(MerkleBlock::async_from_binary(reader).await?),
    //         NOTFOUND => P2PMessage::NotFound(Inv::async_from_binary(reader).await?),
    //         PING => P2PMessage::Ping(Ping::async_from_binary(reader).await?),
    //         PONG => P2PMessage::Pong(Ping::async_from_binary(reader).await?),
    //         PROTOCONF => P2PMessage::Protoconf(Protoconf::async_from_binary(reader).await?),
    //         REJECT => P2PMessage::Reject(Reject::async_from_binary(reader).await?),
    //         SENDCMPCT => P2PMessage::SendCmpct(SendCmpct::async_from_binary(reader).await?),
    //         SENDHEADERS => P2PMessage::SendHeaders,
    //         TX => P2PMessage::Tx(Tx::async_from_binary(reader).await?),
    //         VERACK => P2PMessage::Verack,
    //         VERSION => P2PMessage::Version(Version::async_from_binary(reader).await?),
    //         _ => {
    //             if header.payload_size == 0 {
    //                 trace!(
    //                     "received unknown command={:?} with empty payload",
    //                     std::str::from_utf8(&header.command).unwrap()
    //                 );
    //                 P2PMessage::Unknown(
    //                     format!(
    //                         "Unknown command: {}",
    //                         std::str::from_utf8(&header.command).unwrap()
    //                     ),
    //                     0,
    //                 )
    //             } else {
    //                 let mut v = vec![0u8; header.payload_size as usize];
    //                 reader.read_exact(&mut v).await?;
    //                 trace!(
    //                     "received unknown command={:?} with payload size: {}",
    //                     std::str::from_utf8(&header.command).unwrap(),
    //                     header.payload_size
    //                 );
    //                 P2PMessage::Unknown(
    //                     format!(
    //                         "Unknown command: {:?}, payload size {}",
    //                         std::str::from_utf8(&header.command).unwrap(),
    //                         header.payload_size
    //                     ),
    //                     header.payload_size as usize,
    //                 )
    //             }
    //         }
    //     };
    //     if msg.size() < header.payload_size as usize {
    //         if header.command != VERSION {
    //             // todo: 70016 version message is larger. dont report on it. remove this when we support 70016
    //             warn!(
    //                 "received larger payload than msg: command={:?}, payload size={}, msg size={}",
    //                 std::str::from_utf8(&header.command).unwrap(),
    //                 header.payload_size,
    //                 msg.size()
    //             );
    //         }
    //         // we've read less bytes than the payload size, we need to read the rest and discard it
    //         let mut v = vec![0u8; header.payload_size as usize - msg.size()];
    //         reader.read_exact(&mut v).await?;
    //     }
    //     Ok(msg)
    // }

    // /// Writes a Bitcoin P2P message with its payload to bytes
    // pub async fn write<W: AsyncWrite + Unpin + Send>(
    //     &self,
    //     writer: &mut W,
    //     config: &ChannelConfig,
    // ) -> Result<()> {
    //     match self {
    //         P2PMessage::Addr(p) => self.write_with_payload(writer, ADDR, config, p).await,
    //         P2PMessage::Block(p) => self.write_with_payload(writer, BLOCK, config, p).await,
    //         P2PMessage::GetAddr => self.write_without_payload(writer, GETADDR, config).await,
    //         P2PMessage::GetBlocks(p) => self.write_with_payload(writer, GETBLOCKS, config, p).await,
    //         P2PMessage::GetData(p) => self.write_with_payload(writer, GETDATA, config, p).await,
    //         P2PMessage::GetHeaders(p) => {
    //             self.write_with_payload(writer, GETHEADERS, config, p).await
    //         }
    //         P2PMessage::Headers(p) => self.write_with_payload(writer, HEADERS, config, p).await,
    //         P2PMessage::Inv(p) => self.write_with_payload(writer, INV, config, p).await,
    //         P2PMessage::Mempool => self.write_without_payload(writer, MEMPOOL, config).await,
    //         P2PMessage::MerkleBlock(p) => {
    //             self.write_with_payload(writer, MERKLEBLOCK, config, p)
    //                 .await
    //         }
    //         P2PMessage::NotFound(p) => self.write_with_payload(writer, NOTFOUND, config, p).await,
    //         P2PMessage::Ping(p) => self.write_with_payload(writer, PING, config, p).await,
    //         P2PMessage::Pong(p) => self.write_with_payload(writer, PONG, config, p).await,
    //         P2PMessage::Protoconf(p) => self.write_with_payload(writer, PROTOCONF, config, p).await,
    //         P2PMessage::Reject(p) => self.write_with_payload(writer, REJECT, config, p).await,
    //         P2PMessage::SendCmpct(p) => self.write_with_payload(writer, SENDCMPCT, config, p).await,
    //         P2PMessage::SendHeaders => {
    //             self.write_without_payload(writer, SENDHEADERS, config)
    //                 .await
    //         }
    //         P2PMessage::Tx(p) => self.write_with_payload(writer, TX, config, p).await,
    //         P2PMessage::Verack => self.write_without_payload(writer, VERACK, config).await,
    //         P2PMessage::Version(v) => self.write_with_payload(writer, VERSION, config, v).await,
    //         P2PMessage::Unknown(s, _size) => {
    //             let msg = format!("Unknown command: {:?}", s);
    //             Err(Error::BadData(msg))
    //         }
    //     }
    // }

    // /// Get the size of the payload of the message
    // pub fn size(&self) -> usize {
    //     match self {
    //         P2PMessage::Addr(p) => p.async_size(),
    //         P2PMessage::Block(p) => p.async_size(),
    //         P2PMessage::GetAddr => 0,
    //         P2PMessage::GetBlocks(p) => p.async_size(),
    //         P2PMessage::GetData(p) => p.async_size(),
    //         P2PMessage::GetHeaders(p) => p.async_size(),
    //         P2PMessage::Headers(p) => p.async_size(),
    //         P2PMessage::Inv(p) => p.async_size(),
    //         P2PMessage::Mempool => 0,
    //         P2PMessage::MerkleBlock(p) => p.async_size(),
    //         P2PMessage::NotFound(p) => p.async_size(),
    //         P2PMessage::Ping(p) => p.async_size(),
    //         P2PMessage::Pong(p) => p.async_size(),
    //         P2PMessage::Protoconf(p) => p.async_size(),
    //         P2PMessage::Reject(p) => p.async_size(),
    //         P2PMessage::SendCmpct(p) => p.async_size(),
    //         P2PMessage::SendHeaders => 0,
    //         P2PMessage::Tx(p) => p.async_size(),
    //         P2PMessage::Verack => 0,
    //         P2PMessage::Version(v) => v.async_size(),
    //         P2PMessage::Unknown(_s, size) => *size,
    //     }
    // }

    // /// Write a P2P message that does not have a payload
    // async fn write_without_payload<W: AsyncWrite + Unpin + Send>(
    //     &self,
    //     writer: &mut W,
    //     command: [u8; 12],
    //     config: &ChannelConfig,
    // ) -> Result<()> {
    //     let header = P2PMessageHeader {
    //         magic: config.magic,
    //         command,
    //         payload_size: 0,
    //         checksum: NO_CHECKSUM,
    //     };
    //     let v = header.to_binary_buf()?;
    //     let _ = writer.write(&v).await?;
    //     Ok(())
    // }

    // /// Write a P2P message that has a payload
    // async fn write_with_payload<W, X>(
    //     &self,
    //     writer: &mut W,
    //     command: [u8; 12],
    //     config: &ChannelConfig,
    //     payload: &X,
    // ) -> Result<()>
    // where
    //     W: AsyncWrite + Unpin + Send,
    //     X: AsyncEncodable,
    // {
    //     if config.protocol_version >= 70016 && payload.encoded_size() > 0xffffffff {
    //         // we should use the extended message header
    //         if command != BLOCK {
    //             return Err(Error::BadData("payload too large".to_string()));
    //         }
    //         let header = P2PMessageHeader {
    //             magic: config.magic,
    //             command,
    //             payload_size: payload.async_size() as u64,
    //             checksum: ZERO_CHECKSUM,
    //         };
    //         header.async_to_binary(writer).await?;
    //         payload.async_to_binary(writer).await?;
    //         return Ok(());
    //     }
    //     let buf = payload.to_binary_buf()?;
    //     let hash = Hash::sha256d(&buf);
    //     let header = P2PMessageHeader {
    //         magic: config.magic,
    //         command,
    //         payload_size: buf.len() as u64,
    //         checksum: hash.hash[..4].try_into().unwrap(),
    //     };
    //     header.async_to_binary(writer).await?;
    //     let _ = writer.write(&buf).await?;
    //     Ok(())
    // }
}

impl fmt::Debug for P2PMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            P2PMessage::Addr(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Block(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::GetAddr => f.write_str("GetAddr"),
            P2PMessage::GetBlocks(p) => f
                .debug_struct("GetBlocks")
                .field("version", &p.version)
                .field("block_locator_hashes", &p.block_locator_hashes)
                .field("hash_stop", &p.hash_stop)
                .finish(),
            P2PMessage::GetData(p) => f.debug_struct("GetData").field("inv", &p).finish(),
            P2PMessage::GetHeaders(p) => f
                .debug_struct("GetHeaders")
                .field("version", &p.version)
                .field("block_locator_hashes", &p.block_locator_hashes)
                .field("hash_stop", &p.hash_stop)
                .finish(),
            P2PMessage::Headers(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Inv(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Mempool => f.write_str("Mempool"),
            P2PMessage::MerkleBlock(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::NotFound(p) => f.debug_struct("NotFound").field("inv", &p).finish(),
            P2PMessage::Ping(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Pong(p) => f.debug_struct("Pong").field("nonce", &p.nonce).finish(),
            P2PMessage::Protoconf(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Reject(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::SendCmpct(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::SendHeaders => f.write_str("SendHeaders"),
            P2PMessage::Tx(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Verack => f.write_str("Verack"),
            P2PMessage::Version(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Unknown(p, _size) => f.write_str(&format!("{:#?}", p)),
        }
    }
}

impl fmt::Display for P2PMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            P2PMessage::Addr(p) => f.write_str(&format!("{}", p)),
            P2PMessage::Block(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::GetAddr => f.write_str("GetAddr"),
            P2PMessage::GetBlocks(p) => f
                .debug_struct("GetBlocks")
                .field("version", &p.version)
                .field("block_locator_hashes", &p.block_locator_hashes)
                .field("hash_stop", &p.hash_stop)
                .finish(),
            P2PMessage::GetData(p) => f.debug_struct("GetData").field("inv", &p).finish(),
            P2PMessage::GetHeaders(p) => f
                .debug_struct("GetHeaders")
                .field("version", &p.version)
                .field("block_locator_hashes", &p.block_locator_hashes)
                .field("hash_stop", &p.hash_stop)
                .finish(),
            P2PMessage::Headers(p) => f.write_str(&format!("{}", p)),
            P2PMessage::Inv(p) => f.write_str(&format!("{}", p)),
            P2PMessage::Mempool => f.write_str("Mempool"),
            P2PMessage::MerkleBlock(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::NotFound(p) => f.debug_struct("NotFound").field("inv", &p).finish(),
            P2PMessage::Ping(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Pong(p) => f.debug_struct("Pong").field("nonce", &p.nonce).finish(),
            P2PMessage::Protoconf(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Reject(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::SendCmpct(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::SendHeaders => f.write_str("SendHeaders"),
            P2PMessage::Tx(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Verack => f.write_str("Verack"),
            P2PMessage::Version(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Unknown(p, _size) => f.write_str(&format!("{:#?}", p)),
        }
    }
}

/// We define several different types of P2P Messages
/// These types will be expanded as I flesh out the implementation
pub enum P2PMessageType {
    /// A Data message contains Bitcoin information
    Data,
    /// A ConnectionControl message configures the connection with the peer
    ConnectionControl,
}

impl From<&P2PMessage> for P2PMessageType {
    fn from(value: &P2PMessage) -> Self {
        match value {
            P2PMessage::Addr(_) => Data,
            P2PMessage::Block(_) => Data,
            P2PMessage::GetAddr => Data,
            P2PMessage::GetBlocks(_) => Data,
            P2PMessage::GetData(_) => Data,
            P2PMessage::GetHeaders(_) => Data,
            P2PMessage::Headers(_) => Data,
            P2PMessage::Inv(_) => Data,
            P2PMessage::Mempool => Data,
            P2PMessage::MerkleBlock(_) => Data,
            P2PMessage::NotFound(_) => Data,
            P2PMessage::Ping(_) => ConnectionControl,
            P2PMessage::Pong(_) => ConnectionControl,
            P2PMessage::Protoconf(_) => ConnectionControl,
            P2PMessage::Reject(_) => ConnectionControl,
            P2PMessage::SendCmpct(_) => ConnectionControl,
            P2PMessage::SendHeaders => ConnectionControl,
            P2PMessage::Tx(_) => Data,
            P2PMessage::Verack => ConnectionControl,
            P2PMessage::Version(_) => ConnectionControl,
            P2PMessage::Unknown(_, _) => Data,
        }
    }
}

impl From<Arc<P2PMessage>> for P2PMessageType {
    fn from(value: Arc<P2PMessage>) -> Self {
        match *value {
            P2PMessage::Addr(_) => Data,
            P2PMessage::Block(_) => Data,
            P2PMessage::GetAddr => Data,
            P2PMessage::GetBlocks(_) => Data,
            P2PMessage::GetData(_) => Data,
            P2PMessage::GetHeaders(_) => Data,
            P2PMessage::Headers(_) => Data,
            P2PMessage::Inv(_) => Data,
            P2PMessage::Mempool => Data,
            P2PMessage::MerkleBlock(_) => Data,
            P2PMessage::NotFound(_) => Data,
            P2PMessage::Ping(_) => ConnectionControl,
            P2PMessage::Pong(_) => ConnectionControl,
            P2PMessage::Protoconf(_) => ConnectionControl,
            P2PMessage::Reject(_) => ConnectionControl,
            P2PMessage::SendCmpct(_) => ConnectionControl,
            P2PMessage::SendHeaders => ConnectionControl,
            P2PMessage::Tx(_) => Data,
            P2PMessage::Verack => ConnectionControl,
            P2PMessage::Version(_) => ConnectionControl,
            P2PMessage::Unknown(_, _) => Data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::{BlockHeader, Hash, Outpoint, Script, Tx, TxInput, TxOutput};
    use crate::p2p::messages::inv::{InvItem, InvType};
    use crate::p2p::messages::reject::REJECT_INVALID;
    use crate::p2p::messages::NodeAddr;
    use crate::p2p::params::PROTOCOL_VERSION;
    use crate::util::epoch_secs;
    use hex::FromHex;
    use std::io::Cursor;
    use std::net::{IpAddr, Ipv6Addr};

    // #[tokio::test]
    // async fn write_read() {
    //     let magic = [7, 8, 9, 0];
    //     let mut config = ChannelConfig::default();
    //     config.magic = magic;
    //
    //     // Addr
    //     let mut v = Vec::new();
    //     let a = NodeAddr {
    //         timestamp: 700,
    //         services: 900,
    //         ip: IpAddr::from(Ipv6Addr::from([
    //             0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 9, 8, 7, 6, 5,
    //         ])),
    //         port: 4000,
    //     };
    //     let p = Addr { addrs: vec![a] };
    //     let m = P2PMessage::Addr(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // Block
    //     let mut v = Vec::new();
    //     let p = Block {
    //         header: BlockHeader {
    //             version: 0x00000001,
    //             prev_hash: Hash::from(
    //                 "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234",
    //             ),
    //             merkle_root: Hash::from(
    //                 "2b12fcf1b09288fcaff797d71e950e71ae42b91e8bdb2304758dfcffc2b620e3",
    //             ),
    //             timestamp: 0x4dd7f5c7,
    //             bits: 0x1a44b9f2,
    //             nonce: 0x9546a142,
    //         },
    //         transactions: vec![
    //             Tx {
    //                 version: 0x44556677,
    //                 inputs: vec![TxInput {
    //                     outpoint: Outpoint {
    //                         tx_hash: Hash::from(
    //                             "2b12fcf1b09288fcaff797d71e950e71ae42b91e8bdb2304758dfcffc2b620e3",
    //                         ),
    //                         index: 3,
    //                     },
    //                     script: Script::from(vec![5; 5]),
    //                     sequence: 2,
    //                 }],
    //                 outputs: vec![TxOutput {
    //                     value: 42,
    //                     script: Script::from(vec![9; 21]),
    //                 }],
    //                 lock_time: 0x12ff34aa,
    //             },
    //             Tx {
    //                 version: 0x99881122,
    //                 inputs: vec![TxInput {
    //                     outpoint: Outpoint {
    //                         tx_hash: Hash::from(
    //                             "2b12fcf1b09288fcaff797d71e950e71ae42b91e8bdb2304758dfcffc2b620e3",
    //                         ),
    //                         index: 4,
    //                     },
    //                     script: Script::from(vec![4; 4]),
    //                     sequence: 3,
    //                 }],
    //                 outputs: vec![TxOutput {
    //                     value: 43,
    //                     script: Script::from(vec![10; 22]),
    //                 }],
    //                 lock_time: 0x44550011,
    //             },
    //         ],
    //     };
    //     let m = P2PMessage::Block(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // GetAddr
    //     let mut v = Vec::new();
    //     let m = P2PMessage::GetAddr;
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // GetBlocks
    //     let mut v = Vec::new();
    //     let p = BlockLocator {
    //         version: 567,
    //         block_locator_hashes: vec![
    //             Hash::from("2b12fcf1b09288fcaff797d71e950e71ae42b91e8bdb2304758dfcffc2b620e3"),
    //             Hash::from("abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234"),
    //         ],
    //         hash_stop: Hash::from(
    //             "0b5a8ca2ce1e9a761ae41fa7dcf93973c81851b38e20a7e8f3756f02cdc8e66f",
    //         ),
    //     };
    //     let m = P2PMessage::GetBlocks(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // GetData
    //     let mut v = Vec::new();
    //     let p = Inv {
    //         objects: vec![InvItem {
    //             obj_type: InvType::Tx,
    //             hash: Hash::from(
    //                 "0b5a8ca2ce1e9a761ae41fa7dcf93973c81851b38e20a7e8f3756f02cdc8e66f",
    //             ),
    //         }],
    //     };
    //     let m = P2PMessage::GetData(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // GetHeaders
    //     let mut v = Vec::new();
    //     let p = BlockLocator {
    //         version: 345,
    //         block_locator_hashes: vec![
    //             Hash::from("2b12fcf1b09288fcaff797d71e950e71ae42b91e8bdb2304758dfcffc2b620e3"),
    //             Hash::from("abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234"),
    //         ],
    //         hash_stop: Hash::from(
    //             "0b5a8ca2ce1e9a761ae41fa7dcf93973c81851b38e20a7e8f3756f02cdc8e66f",
    //         ),
    //     };
    //     let m = P2PMessage::GetHeaders(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // Headers
    //     let mut v = Vec::new();
    //     let bh_bin = Vec::from_hex("00405324d8facaf19ce3efc5f6b3fbdc1cb1f5369a56c3de3e50280300000000000000002742bdb5930e5bf24be6e7521ceeecf6d3199871e2a6438f54cb5fd95d3f5139a38d90653c5808186eac9b4c").unwrap();
    //     let p = Headers {
    //         headers: vec![BlockHeader::from_binary_buf(bh_bin.as_slice()).unwrap()],
    //     };
    //     let m = P2PMessage::Headers(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // Inv
    //     let mut v = Vec::new();
    //     let p = Inv {
    //         objects: vec![InvItem {
    //             obj_type: InvType::Tx,
    //             hash: Hash::from(
    //                 "00000000000000000538178e5c48e51e271e009c31d3854886d29328fa0aa037",
    //             ),
    //         }],
    //     };
    //     let m = P2PMessage::Inv(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // Mempool
    //     let mut v = Vec::new();
    //     let m = P2PMessage::Mempool;
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // MerkleBlock
    //     let mut v = Vec::new();
    //     let p = MerkleBlock {
    //         header: BlockHeader {
    //             version: 12345,
    //             prev_hash: Hash::from_hex(
    //                 "7766009988776600998877660099887766009988776600998877660099887766",
    //             )
    //             .unwrap(),
    //             merkle_root: Hash::from_hex(
    //                 "2211554433221155443322115544332211554433221155443322115544332211",
    //             )
    //             .unwrap(),
    //             timestamp: 66,
    //             bits: 4488,
    //             nonce: 9999,
    //         },
    //         total_transactions: 14,
    //         hashes: vec![
    //             Hash::from("2b12fcf1b09288fcaff797d71e950e71ae42b91e8bdb2304758dfcffc2b620e3"),
    //             Hash::from("abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234"),
    //         ],
    //         flags: vec![24, 125, 199],
    //     };
    //     let m = P2PMessage::MerkleBlock(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // NotFound
    //     let mut v = Vec::new();
    //     let p = Inv {
    //         objects: vec![InvItem {
    //             obj_type: InvType::Tx,
    //             hash: Hash::from(
    //                 "0b5a8ca2ce1e9a761ae41fa7dcf93973c81851b38e20a7e8f3756f02cdc8e66f",
    //             ),
    //         }],
    //     };
    //     let m = P2PMessage::NotFound(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // Ping
    //     let mut v = Vec::new();
    //     let p = Ping { nonce: 7890 };
    //     let m = P2PMessage::Ping(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // Pong
    //     let mut v = Vec::new();
    //     let p = Ping { nonce: 7890 };
    //     let m = P2PMessage::Pong(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // Reject
    //     let mut v = Vec::new();
    //     let p = Reject {
    //         message: "getaddr\0\0\0\0\0".to_string(),
    //         code: REJECT_INVALID,
    //         reason: "womp womp".to_string(),
    //         data: vec![],
    //     };
    //     let m = P2PMessage::Reject(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // SendCmpct
    //     let mut v = Vec::new();
    //     let p = SendCmpct {
    //         enable: 1,
    //         version: 1,
    //     };
    //     let m = P2PMessage::SendCmpct(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // SendHeaders
    //     let mut v = Vec::new();
    //     let m = P2PMessage::SendHeaders;
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // Tx
    //     let mut v = Vec::new();
    //     let p = Tx {
    //         version: 0x44556677,
    //         inputs: vec![TxInput {
    //             outpoint: Outpoint {
    //                 tx_hash: Hash::from(
    //                     "0b5a8ca2ce1e9a761ae41fa7dcf93973c81851b38e20a7e8f3756f02cdc8e66f",
    //                 ),
    //                 index: 3,
    //             },
    //             script: Script::from(vec![7u8; 7]),
    //             sequence: 2,
    //         }],
    //         outputs: vec![TxOutput {
    //             value: 42,
    //             script: Script::from(vec![8u8; 8]),
    //         }],
    //         lock_time: 0x12ff34aa,
    //     };
    //     let m = P2PMessage::Tx(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // Verack
    //     let mut v = Vec::new();
    //     let m = P2PMessage::Verack;
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    //
    //     // Version
    //     let mut v = Vec::new();
    //     let p = Version {
    //         version: PROTOCOL_VERSION,
    //         services: 77,
    //         timestamp: epoch_secs(),
    //         recv_addr: NodeAddr {
    //             ..Default::default()
    //         },
    //         tx_addr: NodeAddr {
    //             ..Default::default()
    //         },
    //         nonce: 99,
    //         user_agent: "dummy".to_string(),
    //         start_height: 22,
    //         relay: true,
    //     };
    //     let m = P2PMessage::Version(p);
    //     m.write(&mut v, &config).await.unwrap();
    //     assert_eq!(
    //         P2PMessage::read(&mut Cursor::new(&v), &config)
    //             .await
    //             .unwrap(),
    //         m
    //     );
    // }

    // #[test]
    // #[should_panic]
    // fn write_other_errors() {
    //     let mut v = Vec::new();
    //     let m = Message::Other("Unknown message".to_string());
    //     m.write(&mut v, [7, 8, 9, 0]).unwrap();
    // }

    // #[test]
    // fn read_other() {
    //     let magic = [7, 8, 9, 0];
    //     let command = *b"unknowncmd\0\0";
    //     let header = MessageHeader {
    //         magic,
    //         command,
    //         payload_size: 0,
    //         checksum: NO_CHECKSUM,
    //     };
    //     let mut v = Vec::new();
    //     header.write(&mut v).unwrap();
    //     let mut cursor = Cursor::new(&v);
    //     let m = Message::read(&mut cursor, magic).unwrap();
    //     if let Message::Other(_) = m {
    //         // Success
    //     } else {
    //         assert!(false);
    //     }
    // }
}
