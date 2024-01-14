use crate::{Error, Result};
use std::fmt;
use std::io::Cursor;
use log::{trace, warn};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::bitcoin::Encodable;
use crate::bitcoin::hash::Hash;
use crate::p2p::messages::messages::commands::{GETADDR, MEMPOOL, SENDHEADERS, VERACK, VERSION};
use crate::p2p::messages::messages::P2PMessageType::{ConnectionControl, Data};
use crate::p2p::messages::msg_header::P2PMessageHeader;
use crate::p2p::messages::Version;

// based on code imported from rust-sv but substantially modified

// I wont be implementing the FEEFILTER related messages. These aren't scalable. As unknown messages,
// they will be ignored.
//
// The Bitcoin P2P protocol is a message based protocol, where each message is a chunk of contiguous data.
// Most of the messages represent a single fact but there are a few exceptions. However, on closer examination
// most of the messages that contain lists of items attribute significance to the list itself and the position
// of items within the list.
//      ADDR - this is an exception, there's no significance to the ordering of the list
//      INV (tx) - transactions are supposed to be in this list in order of dependence, with parent transactions
//                  preceding children. This can not be relied upon though.
//      GETDATA - like the inv, the order of transactions in this list is significant.
//      NOTFOUND - ordering not significant
//      GETBLOCKS - specifically relevant
//      GETHEADERS - specifically relevant
//      HEADERS - specifically relevant
// Given the significance of ordering within these messages, there is no benefit from breaking the messages up
// into smaller parts and streaming those parts.
//
// Most of the messages are also reasonably small, the messages are quickly transferred between
// peers. There is also a maximum message size which limits the amount of memory that will be allocated to deal
// with a message in its entirety.
//
// There is one exception to this, and one potential exception that I need to look into.
//
// The known exception is the BLOCK message, which transfers a block in its entirety. With large blocks
// (up to 4GB at the time of writing), this can cause a significant memory issue and we will eventually have special
// handling for this message.  <IMPROVEMENT - do this>
//
// The TX message is a potential concern. Transactions can get large but there is also a policy on the SV Node
// regarding the maximum size of transactions.  <IMPROVEMENT - check this and adjust>
//
// Each message consists of a header and a payload. The header starts with a set of "magic" bytes that identify both
// the blockchain to which these messages apply (mainnet, testnet, regtest, stn). The header also contains the size of
// the payload. Our code needs to protect against cases where the payload size specified in the header is incorrect.

// Given the above, I'm going to treat every packet in its entirety by default, and not implement a streaming read
// trait. The exception to this is the Block, I will add a streaming interface to this at some point.


/// Checksum to use when there is an empty payload.
pub const NO_CHECKSUM: [u8; 4] = [0x5d, 0xf6, 0xe0, 0xe2];

/// Default max message payload size (32MB).
pub const DEFAULT_MAX_PAYLOAD_SIZE: u64 = 0x02000000;

/// Message commands for the header
mod commands {
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
    // Addr(Addr),
    // Block(Block),
    GetAddr,
    // GetBlocks(BlockLocator),
    // GetData(Inv),
    // GetHeaders(BlockLocator),
    // Headers(Headers),
    // Inv(Inv),
    Mempool,
    // MerkleBlock(MerkleBlock),
    // NotFound(Inv),
    // Partial(MessageHeader),
    // Ping(Ping),
    // Pong(Ping),
    // Reject(Reject),
    SendHeaders,
    // SendCmpct(SendCmpct),
    // Tx(Tx),
    Verack,
    Version(Version),
    Unknown(String),
}

impl P2PMessage {
    /// Read a full P2P message from the reader
    pub async fn read<R: AsyncRead + Unpin + Send>(reader: &mut R, magic: [u8; 4], max_size: u64) -> Result<Self> {
        let mut v = vec![0u8; P2PMessageHeader::SIZE];
        match reader.read_exact(&mut v).await {
            Ok(_) => {},
            Err(e) => {
                trace!("Error reading message header: {}", e);
                let msg = format!("Error reading message header: {}", e);
                return Err(Error::BadData(msg));
            },
        }
        let header = P2PMessageHeader::decode(&mut Cursor::new(&v))?;
        trace!("P2PMessage::read() - header: {:?}", header);
        match header.validate(magic, max_size) {
            Ok(_) => {},
            Err(e) => {
                trace!("P2PMessage::read() - Error validating message header: {}, header: {:?}", e, header);
                let msg = format!("Error validating message header: {}", e);
                return Err(Error::BadData(msg));
            },
        }
        // payload size has been checked for max limit in header.validate()
        let mut payload = vec![0u8; header.payload_size as usize];
        if header.payload_size > 0 {
            let _ = reader.read_exact(&mut payload).await?;
        }
        let mut p_cursor = Cursor::new(&payload);
        let msg= match header.command {
            GETADDR => P2PMessage::GetAddr,
            MEMPOOL => P2PMessage::Mempool,
            SENDHEADERS => P2PMessage::SendHeaders,
            VERACK => P2PMessage::Verack,
            VERSION => P2PMessage::Version(Version::decode(&mut p_cursor).unwrap()),
            _ => {
                if header.payload_size == 0 {
                    trace!("received unknown command={:?} with empty payload", std::str::from_utf8(&header.command).unwrap());
                    P2PMessage::Unknown(format!("Unknown command: {}", std::str::from_utf8(&header.command).unwrap()))
                } else {
                    trace!("received unknown command={:?} with payload size: {}", std::str::from_utf8(&header.command).unwrap(), header.payload_size);
                    P2PMessage::Unknown(format!("Unknown command: {:?}, payload size {}", std::str::from_utf8(&header.command).unwrap(), header.payload_size))
                }
            },
        };
        // if msg.size() < header.payload_size as usize {       todo
        //     warn!("received larger payload than msg, ignoring rest: command={:?}, reported size={}, received size={}", std::str::from_utf8(&header.command).unwrap(), header.payload_size, payload_size);
        //     let mut v = vec![0u8; header.payload_size as usize - payload_size];
        //     let _ = reader.read_exact(&mut v).await.unwrap();
        // }
        Ok(msg)
    }

    /// Writes a Bitcoin P2P message with its payload to bytes
    pub async fn write<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W, magic: [u8; 4]) -> Result<()> {
        match self {
            // P2PMessage::Addr(p) => write_with_payload(writer, ADDR, p, magic),
            // P2PMessage::Block(p) => write_with_payload(writer, BLOCK, p, magic),
            P2PMessage::GetAddr => self.write_without_payload(writer, GETADDR, magic).await,
            // P2PMessage::GetBlocks(p) => write_with_payload(writer, GETBLOCKS, p, magic),
            // P2PMessage::GetData(p) => write_with_payload(writer, GETDATA, p, magic),
            // P2PMessage::GetHeaders(p) => write_with_payload(writer, GETHEADERS, p, magic),
            // P2PMessage::Headers(p) => write_with_payload(writer, HEADERS, p, magic),
            P2PMessage::Mempool => self.write_without_payload(writer, MEMPOOL, magic).await,
            // P2PMessage::MerkleBlock(p) => write_with_payload(writer, MERKLEBLOCK, p, magic),
            // P2PMessage::NotFound(p) => write_with_payload(writer, NOTFOUND, p, magic),
            // P2PMessage::Inv(p) => write_with_payload(writer, INV, p, magic),
            // P2PMessage::Ping(p) => write_with_payload(writer, PING, p, magic),
            // P2PMessage::Pong(p) => write_with_payload(writer, PONG, p, magic),
            // P2PMessage::Reject(p) => write_with_payload(writer, REJECT, p, magic),
            P2PMessage::SendHeaders => self.write_without_payload(writer, SENDHEADERS, magic).await,
            // P2PMessage::SendCmpct(p) => write_with_payload(writer, SENDCMPCT, p, magic),
            // P2PMessage::Tx(p) => write_with_payload(writer, TX, p, magic),
            P2PMessage::Verack => self.write_without_payload(writer, VERACK, magic).await,
            P2PMessage::Version(v) => self.write_with_payload(writer, VERSION, magic, v).await,
            P2PMessage::Unknown(s) => {
                let msg = format!("Unknown command: {:?}", s);
                return Err(Error::BadData(msg));
            }
        }
    }

    /// Write a P2P message that does not have a payload
    async fn write_without_payload<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W, command: [u8; 12], magic: [u8; 4]) -> Result<()> {
        let header = P2PMessageHeader {
            magic,
            command,
            payload_size: 0,
            checksum: NO_CHECKSUM,
        };
        let v = header.encode()?;
        let _ = writer.write(&v).await?;
        Ok(())
    }

    /// Write a P2P message that has a payload
    async fn write_with_payload<W, X>(&self, writer: &mut W, command: [u8; 12], magic: [u8; 4], payload: &X) -> Result<()>
        where W: AsyncWrite + Unpin + Send,
            X: Encodable,
    {
        let buf = payload.encode()?;
        let hash = Hash::sha256d(&buf);
        let header = P2PMessageHeader {
            magic,
            command,
            payload_size: buf.len() as u32,
            checksum: hash.hash[..4].try_into().unwrap(),
        };
        let v = header.encode()?;
        let _ = writer.write(&v).await?;
        let _ = writer.write(&buf).await;
        Ok(())
    }
}

impl fmt::Debug for P2PMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // Message::Addr(p) => f.write_str(&format!("{:#?}", p)),
            // Message::Block(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::GetAddr => f.write_str("GetAddr"),
            // Message::GetBlocks(p) => f
            //     .debug_struct("GetBlocks")
            //     .field("version", &p.version)
            //     .field("block_locator_hashes", &p.block_locator_hashes)
            //     .field("hash_stop", &p.hash_stop)
            //     .finish(),
            // Message::GetData(p) => f.debug_struct("GetData").field("inv", &p).finish(),
            // Message::GetHeaders(p) => f
            //     .debug_struct("GetHeaders")
            //     .field("version", &p.version)
            //     .field("block_locator_hashes", &p.block_locator_hashes)
            //     .field("hash_stop", &p.hash_stop)
            //     .finish(),
            // Message::Headers(p) => f.write_str(&format!("{:#?}", p)),
            // Message::Inv(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Mempool => f.write_str("Mempool"),
            // Message::MerkleBlock(p) => f.write_str(&format!("{:#?}", p)),
            // Message::NotFound(p) => f.debug_struct("NotFound").field("inv", &p).finish(),
            // Message::Ping(p) => f.write_str(&format!("{:#?}", p)),
            // Message::Pong(p) => f.debug_struct("Pong").field("nonce", &p.nonce).finish(),
            // Message::Reject(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::SendHeaders => f.write_str("SendHeaders"),
            // Message::SendCmpct(p) => f.write_str(&format!("{:#?}", p)),
            // Message::Tx(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Verack => f.write_str("Verack"),
            P2PMessage::Version(p) => f.write_str(&format!("{:#?}", p)),
            P2PMessage::Unknown(p) => f.write_str(&format!("{:#?}", p)),
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

impl From<P2PMessage> for P2PMessageType {
    fn from(value: P2PMessage) -> Self {
        match value {
            P2PMessage::GetAddr => Data,
            P2PMessage::Mempool => Data,
            P2PMessage::SendHeaders => Data,
            P2PMessage::Verack => ConnectionControl,
            P2PMessage::Version(_) => ConnectionControl,
            P2PMessage::Unknown(_) => Data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use crate::p2p::messages::NodeAddr;
    use crate::p2p::messages::version::PROTOCOL_VERSION;
    use crate::util::epoch_secs;

    #[tokio::test]
    async fn write_read() {
        let magic = [7, 8, 9, 0];

        // Addr
        // let mut v = Vec::new();
        // let a = NodeAddrEx {
        //     last_connected_time: 700,
        //     addr: NodeAddr {
        //         services: 900,
        //         ip: Ipv6Addr::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 9, 8, 7, 6, 5]),
        //         port: 4000,
        //     },
        // };
        // let p = Addr { addrs: vec![a] };
        // let m = Message::Addr(p);
        // m.write(&mut v, magic).unwrap();
        // assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);

        // Block
        // let mut v = Vec::new();
        // let p = Block {
        //     header: BlockHeader {
        //         version: 0x00000001,
        //         prev_hash: Hash256::decode(
        //             "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234",
        //         )
        //             .unwrap(),
        //         merkle_root: Hash256::decode(
        //             "2b12fcf1b09288fcaff797d71e950e71ae42b91e8bdb2304758dfcffc2b620e3",
        //         )
        //             .unwrap(),
        //         timestamp: 0x4dd7f5c7,
        //         bits: 0x1a44b9f2,
        //         nonce: 0x9546a142,
        //     },
        //     txns: vec![
        //         Tx {
        //             version: 0x44556677,
        //             inputs: vec![TxIn {
        //                 prev_output: OutPoint {
        //                     hash: Hash256([5; 32]),
        //                     index: 3,
        //                 },
        //                 unlock_script: Script(vec![5; 5]),
        //                 sequence: 2,
        //             }],
        //             outputs: vec![TxOut {
        //                 satoshis: 42,
        //                 lock_script: Script(vec![9; 21]),
        //             }],
        //             lock_time: 0x12ff34aa,
        //         },
        //         Tx {
        //             version: 0x99881122,
        //             inputs: vec![TxIn {
        //                 prev_output: OutPoint {
        //                     hash: Hash256([6; 32]),
        //                     index: 4,
        //                 },
        //                 unlock_script: Script(vec![4; 4]),
        //                 sequence: 3,
        //             }],
        //             outputs: vec![TxOut {
        //                 satoshis: 43,
        //                 lock_script: Script(vec![10; 22]),
        //             }],
        //             lock_time: 0x44550011,
        //         },
        //     ],
        // };
        // let m = Message::Block(p);
        // m.write(&mut v, magic).unwrap();
        // assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);

        // GetAddr
        let mut v = Vec::new();
        let m = P2PMessage::GetAddr;
        m.write(&mut v, magic).await.unwrap();
        assert_eq!(P2PMessage::read(&mut Cursor::new(&v), magic, DEFAULT_MAX_PAYLOAD_SIZE).await.unwrap(), m);

    //     // GetBlocks
    //     let mut v = Vec::new();
    //     let p = BlockLocator {
    //         version: 567,
    //         block_locator_hashes: vec![Hash256([3; 32]), Hash256([4; 32])],
    //         hash_stop: Hash256([6; 32]),
    //     };
    //     let m = Message::GetBlocks(p);
    //     m.write(&mut v, magic).unwrap();
    //     assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
    //
    //     // GetData
    //     let mut v = Vec::new();
    //     let p = Inv {
    //         objects: vec![InvVect {
    //             obj_type: INV_VECT_TX,
    //             hash: Hash256([0; 32]),
    //         }],
    //     };
    //     let m = Message::GetData(p);
    //     m.write(&mut v, magic).unwrap();
    //     assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
    //
    //     // GetHeaders
    //     let mut v = Vec::new();
    //     let p = BlockLocator {
    //         version: 345,
    //         block_locator_hashes: vec![Hash256([1; 32]), Hash256([2; 32])],
    //         hash_stop: Hash256([3; 32]),
    //     };
    //     let m = Message::GetHeaders(p);
    //     m.write(&mut v, magic).unwrap();
    //     assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
    //
    //     // Headers
    //     let mut v = Vec::new();
    //     let p = Headers {
    //         headers: vec![BlockHeader {
    //             ..default::default()
    //         }],
    //     };
    //     let m = Message::Headers(p);
    //     m.write(&mut v, magic).unwrap();
    //     assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);

        // Mempool
        let mut v = Vec::new();
        let m = P2PMessage::Mempool;
        m.write(&mut v, magic).await.unwrap();
        assert_eq!(P2PMessage::read(&mut Cursor::new(&v), magic, DEFAULT_MAX_PAYLOAD_SIZE).await.unwrap(), m);

    //     // MerkleBlock
    //     let mut v = Vec::new();
    //     let p = MerkleBlock {
    //         header: BlockHeader {
    //             version: 12345,
    //             prev_hash: Hash256::decode(
    //                 "7766009988776600998877660099887766009988776600998877660099887766",
    //             )
    //                 .unwrap(),
    //             merkle_root: Hash256::decode(
    //                 "2211554433221155443322115544332211554433221155443322115544332211",
    //             )
    //                 .unwrap(),
    //             timestamp: 66,
    //             bits: 4488,
    //             nonce: 9999,
    //         },
    //         total_transactions: 14,
    //         hashes: vec![Hash256([1; 32]), Hash256([3; 32]), Hash256([5; 32])],
    //         flags: vec![24, 125, 199],
    //     };
    //     let m = Message::MerkleBlock(p);
    //     m.write(&mut v, magic).unwrap();
    //     assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
    //
    //     // NotFound
    //     let mut v = Vec::new();
    //     let p = Inv {
    //         objects: vec![InvVect {
    //             obj_type: INV_VECT_TX,
    //             hash: Hash256([0; 32]),
    //         }],
    //     };
    //     let m = Message::NotFound(p);
    //     m.write(&mut v, magic).unwrap();
    //     assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
    //
    //     // Inv
    //     let mut v = Vec::new();
    //     let p = Inv {
    //         objects: vec![InvVect {
    //             obj_type: INV_VECT_TX,
    //             hash: Hash256([0; 32]),
    //         }],
    //     };
    //     let m = Message::Inv(p);
    //     m.write(&mut v, magic).unwrap();
    //     assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
    //
    //     // Ping
    //     let mut v = Vec::new();
    //     let p = Ping { nonce: 7890 };
    //     let m = Message::Ping(p);
    //     m.write(&mut v, magic).unwrap();
    //     assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
    //
    //     // Pong
    //     let mut v = Vec::new();
    //     let p = Ping { nonce: 7890 };
    //     let m = Message::Pong(p);
    //     m.write(&mut v, magic).unwrap();
    //     assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
    //
    //     // Reject
    //     let mut v = Vec::new();
    //     let p = Reject {
    //         message: "getaddr\0\0\0\0\0".to_string(),
    //         code: REJECT_INVALID,
    //         reason: "womp womp".to_string(),
    //         data: vec![],
    //     };
    //     let m = Message::Reject(p);
    //     m.write(&mut v, magic).unwrap();
    //     assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);

        // SendHeaders
        let mut v = Vec::new();
        let m = P2PMessage::SendHeaders;
        m.write(&mut v, magic).await.unwrap();
        assert_eq!(P2PMessage::read(&mut Cursor::new(&v), magic, DEFAULT_MAX_PAYLOAD_SIZE).await.unwrap(), m);

    //     // SendCmpct
    //     let mut v = Vec::new();
    //     let p = SendCmpct {
    //         enable: 1,
    //         version: 1,
    //     };
    //     let m = Message::SendCmpct(p);
    //     m.write(&mut v, magic).unwrap();
    //     assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
    //
    //     // Tx
    //     let mut v = Vec::new();
    //     let p = Tx {
    //         version: 0x44556677,
    //         inputs: vec![TxIn {
    //             prev_output: OutPoint {
    //                 hash: Hash256([5; 32]),
    //                 index: 3,
    //             },
    //             unlock_script: Script(vec![7; 7]),
    //             sequence: 2,
    //         }],
    //         outputs: vec![TxOut {
    //             satoshis: 42,
    //             lock_script: Script(vec![8; 8]),
    //         }],
    //         lock_time: 0x12ff34aa,
    //     };
    //     let m = Message::Tx(p);
    //     m.write(&mut v, magic).unwrap();
    //     assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);

        // Verack
        let mut v = Vec::new();
        let m = P2PMessage::Verack;
        m.write(&mut v, magic).await.unwrap();
        assert_eq!(P2PMessage::read(&mut Cursor::new(&v), magic, DEFAULT_MAX_PAYLOAD_SIZE).await.unwrap(), m);

        // Version
        let mut v = Vec::new();
        let p = Version {
            version: PROTOCOL_VERSION,
            services: 77,
            timestamp: epoch_secs(),
            recv_addr: NodeAddr {
                ..Default::default()
            },
            tx_addr: NodeAddr {
                ..Default::default()
            },
            nonce: 99,
            user_agent: "dummy".to_string(),
            start_height: 22,
            relay: true,
        };
        let m = P2PMessage::Version(p);
        m.write(&mut v, magic).await.unwrap();
        assert_eq!(P2PMessage::read(&mut Cursor::new(&v), magic, DEFAULT_MAX_PAYLOAD_SIZE).await.unwrap(), m);
    }

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
