use crate::{Error, Result};
use std::fmt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use crate::bitcoin::Encodable;
use crate::bitcoin::hash::Hash;
use crate::p2p::messages::messages::commands::{GETADDR, MEMPOOL, SENDHEADERS, VERACK, VERSION};
use crate::p2p::messages::msg_header::P2PMessageHeader;
use crate::p2p::messages::Version;

// based on code imported from rust-sv but substantially modified

/// Checksum to use when there is an empty payload
pub const NO_CHECKSUM: [u8; 4] = [0x5d, 0xf6, 0xe0, 0xe2];

/// Default max message payload size (32MB)
pub const DEFAULT_MAX_PAYLOAD_SIZE: u32 = 0x02000000;

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
    pub async fn read<R: AsyncRead + Unpin + Send>(magic: [u8; 4], max_size: u64, reader: &mut R) -> Result<Self> {
        let header = P2PMessageHeader::read(reader).await?;
        header.validate(magic, max_size)?;
        match header.command {
            commands::VERSION => {
                let version = Version::read(reader).await?;
                Ok(P2PMessage::Version(version))
            }
            _ => {
                P2PMessage::read_ignore_payload(header.payload_size as usize, &header.command, reader).await
            }
        }
    }

    /// If we dont recognize the command, we read the payload and ignore it
    async fn read_ignore_payload<R>(num_bytes: usize, command: &[u8; 12], reader: &mut R) -> Result<Self>
        where R: AsyncRead + Unpin + Send,
    {
        let mut v = vec![0u8; 1024];            // read up to 1KB at a time to avoid allocating a huge buffer
        let mut bytes_read = 0;
        while bytes_read < num_bytes {
            let bytes_to_read = std::cmp::min(num_bytes - bytes_read, v.len());
            bytes_read += reader.read_exact(&mut v[..bytes_to_read]).await?;
        }
        let s = format!("Unknown command: {:?}, payload size {}", std::str::from_utf8(command).unwrap(), num_bytes);
        Ok(P2PMessage::Unknown(s))
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
        header.write(writer).await
    }

    /// Write a P2P message that has a payload
    // This code shows very nicely why I have been objecting to the checksum in the message header
    // for the last several years. The checksum is useless and even worse, it is at the beginning of
    // the message. So for every message, we have to serialize the message to bytes, then calculate
    // the checksum and only then can we start sending the message header, followed by the payload.
    // Without the checksum we could just start sending the message as we constructed it.
    // Improvement: There is a cumbersome workaround for blocks, the biggest messages, which I'll
    // get around to implementing at some point.
    async fn write_with_payload<W, X>(&self, writer: &mut W, command: [u8; 12], magic: [u8; 4], payload: &X) -> Result<()>
        where W: AsyncWrite + Unpin + Send,
            X: Encodable,
    {
        let sz = payload.size();
        let mut buf: Vec<u8> = Vec::with_capacity(sz);
        payload.write(&mut buf).await?;
        let hash = Hash::sha256d(&buf);
        let header = P2PMessageHeader {
            magic,
            command,
            payload_size: sz as u32,
            checksum: hash.hash[..4].try_into().unwrap(),
        };
        header.write(writer).await?;
        payload.write(writer).await
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

// fn write_with_payload<T: Serializable<T>>(
//     writer: &mut dyn Write,
//     command: [u8; 12],
//     payload: &dyn Payload<T>,
//     magic: [u8; 4],
// ) -> io::Result<()> {
//     let mut bytes = Vec::with_capacity(payload.size());
//     payload.write(&mut bytes)?;
//     let hash = digest::digest(&digest::SHA256, bytes.as_ref());
//     let hash = digest::digest(&digest::SHA256, &hash.as_ref());
//     let h = &hash.as_ref();
//     let checksum = [h[0], h[1], h[2], h[3]];
//
//     let header = MessageHeader {
//         magic,
//         command,
//         payload_size: payload.size() as u32,
//         checksum: checksum,
//     };
//
//     header.write(writer)?;
//     payload.write(writer)
// }
//
// /// Message payload that is writable to bytes
// pub trait Payload<T>: Serializable<T> + fmt::Debug {
//     fn size(&self) -> usize;
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::messages::block_header::BlockHeader;
//     use crate::messages::inv_vect::{InvVect, INV_VECT_TX};
//     use crate::messages::node_addr::NodeAddr;
//     use crate::messages::node_addr_ex::NodeAddrEx;
//     use crate::messages::out_point::OutPoint;
//     use crate::messages::tx_in::TxIn;
//     use crate::messages::tx_out::TxOut;
//     use crate::messages::version::MIN_SUPPORTED_PROTOCOL_VERSION;
//     use crate::messages::REJECT_INVALID;
//     use crate::script::Script;
//     use crate::util::{secs_since, BloomFilter, Hash256};
//     use std::io::Cursor;
//     use std::net::Ipv6Addr;
//     use std::time::UNIX_EPOCH;
//
//     #[test]
//     fn write_read() {
//         let magic = [7, 8, 9, 0];
//
//         // Addr
//         let mut v = Vec::new();
//         let a = NodeAddrEx {
//             last_connected_time: 700,
//             addr: NodeAddr {
//                 services: 900,
//                 ip: Ipv6Addr::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 9, 8, 7, 6, 5]),
//                 port: 4000,
//             },
//         };
//         let p = Addr { addrs: vec![a] };
//         let m = Message::Addr(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // Block
//         let mut v = Vec::new();
//         let p = Block {
//             header: BlockHeader {
//                 version: 0x00000001,
//                 prev_hash: Hash256::decode(
//                     "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234",
//                 )
//                     .unwrap(),
//                 merkle_root: Hash256::decode(
//                     "2b12fcf1b09288fcaff797d71e950e71ae42b91e8bdb2304758dfcffc2b620e3",
//                 )
//                     .unwrap(),
//                 timestamp: 0x4dd7f5c7,
//                 bits: 0x1a44b9f2,
//                 nonce: 0x9546a142,
//             },
//             txns: vec![
//                 Tx {
//                     version: 0x44556677,
//                     inputs: vec![TxIn {
//                         prev_output: OutPoint {
//                             hash: Hash256([5; 32]),
//                             index: 3,
//                         },
//                         unlock_script: Script(vec![5; 5]),
//                         sequence: 2,
//                     }],
//                     outputs: vec![TxOut {
//                         satoshis: 42,
//                         lock_script: Script(vec![9; 21]),
//                     }],
//                     lock_time: 0x12ff34aa,
//                 },
//                 Tx {
//                     version: 0x99881122,
//                     inputs: vec![TxIn {
//                         prev_output: OutPoint {
//                             hash: Hash256([6; 32]),
//                             index: 4,
//                         },
//                         unlock_script: Script(vec![4; 4]),
//                         sequence: 3,
//                     }],
//                     outputs: vec![TxOut {
//                         satoshis: 43,
//                         lock_script: Script(vec![10; 22]),
//                     }],
//                     lock_time: 0x44550011,
//                 },
//             ],
//         };
//         let m = Message::Block(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // FeeFilter
//         let mut v = Vec::new();
//         let p = FeeFilter { minfee: 1234 };
//         let m = Message::FeeFilter(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // FilterAdd
//         let mut v = Vec::new();
//         let p = FilterAdd { data: vec![15; 45] };
//         let m = Message::FilterAdd(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // FilterClear
//         let mut v = Vec::new();
//         let m = Message::FilterClear;
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // FilterLoad
//         let mut v = Vec::new();
//         let p = FilterLoad {
//             bloom_filter: BloomFilter {
//                 filter: vec![1, 2, 3],
//                 num_hash_funcs: 2,
//                 tweak: 1,
//             },
//             flags: 0,
//         };
//         let m = Message::FilterLoad(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // GetAddr
//         let mut v = Vec::new();
//         let m = Message::GetAddr;
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // GetBlocks
//         let mut v = Vec::new();
//         let p = BlockLocator {
//             version: 567,
//             block_locator_hashes: vec![Hash256([3; 32]), Hash256([4; 32])],
//             hash_stop: Hash256([6; 32]),
//         };
//         let m = Message::GetBlocks(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // GetData
//         let mut v = Vec::new();
//         let p = Inv {
//             objects: vec![InvVect {
//                 obj_type: INV_VECT_TX,
//                 hash: Hash256([0; 32]),
//             }],
//         };
//         let m = Message::GetData(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // GetHeaders
//         let mut v = Vec::new();
//         let p = BlockLocator {
//             version: 345,
//             block_locator_hashes: vec![Hash256([1; 32]), Hash256([2; 32])],
//             hash_stop: Hash256([3; 32]),
//         };
//         let m = Message::GetHeaders(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // Headers
//         let mut v = Vec::new();
//         let p = Headers {
//             headers: vec![BlockHeader {
//                 ..Default::default()
//             }],
//         };
//         let m = Message::Headers(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // Mempool
//         let mut v = Vec::new();
//         let m = Message::Mempool;
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // MerkleBlock
//         let mut v = Vec::new();
//         let p = MerkleBlock {
//             header: BlockHeader {
//                 version: 12345,
//                 prev_hash: Hash256::decode(
//                     "7766009988776600998877660099887766009988776600998877660099887766",
//                 )
//                     .unwrap(),
//                 merkle_root: Hash256::decode(
//                     "2211554433221155443322115544332211554433221155443322115544332211",
//                 )
//                     .unwrap(),
//                 timestamp: 66,
//                 bits: 4488,
//                 nonce: 9999,
//             },
//             total_transactions: 14,
//             hashes: vec![Hash256([1; 32]), Hash256([3; 32]), Hash256([5; 32])],
//             flags: vec![24, 125, 199],
//         };
//         let m = Message::MerkleBlock(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // NotFound
//         let mut v = Vec::new();
//         let p = Inv {
//             objects: vec![InvVect {
//                 obj_type: INV_VECT_TX,
//                 hash: Hash256([0; 32]),
//             }],
//         };
//         let m = Message::NotFound(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // Inv
//         let mut v = Vec::new();
//         let p = Inv {
//             objects: vec![InvVect {
//                 obj_type: INV_VECT_TX,
//                 hash: Hash256([0; 32]),
//             }],
//         };
//         let m = Message::Inv(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // Ping
//         let mut v = Vec::new();
//         let p = Ping { nonce: 7890 };
//         let m = Message::Ping(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // Pong
//         let mut v = Vec::new();
//         let p = Ping { nonce: 7890 };
//         let m = Message::Pong(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // Reject
//         let mut v = Vec::new();
//         let p = Reject {
//             message: "getaddr\0\0\0\0\0".to_string(),
//             code: REJECT_INVALID,
//             reason: "womp womp".to_string(),
//             data: vec![],
//         };
//         let m = Message::Reject(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // SendHeaders
//         let mut v = Vec::new();
//         let m = Message::SendHeaders;
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // SendCmpct
//         let mut v = Vec::new();
//         let p = SendCmpct {
//             enable: 1,
//             version: 1,
//         };
//         let m = Message::SendCmpct(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // Tx
//         let mut v = Vec::new();
//         let p = Tx {
//             version: 0x44556677,
//             inputs: vec![TxIn {
//                 prev_output: OutPoint {
//                     hash: Hash256([5; 32]),
//                     index: 3,
//                 },
//                 unlock_script: Script(vec![7; 7]),
//                 sequence: 2,
//             }],
//             outputs: vec![TxOut {
//                 satoshis: 42,
//                 lock_script: Script(vec![8; 8]),
//             }],
//             lock_time: 0x12ff34aa,
//         };
//         let m = Message::Tx(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // Verack
//         let mut v = Vec::new();
//         let m = Message::Verack;
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//
//         // Version
//         let mut v = Vec::new();
//         let p = Version {
//             version: MIN_SUPPORTED_PROTOCOL_VERSION,
//             services: 77,
//             timestamp: secs_since(UNIX_EPOCH) as i64,
//             recv_addr: NodeAddr {
//                 ..Default::default()
//             },
//             tx_addr: NodeAddr {
//                 ..Default::default()
//             },
//             nonce: 99,
//             user_agent: "dummy".to_string(),
//             start_height: 22,
//             relay: true,
//         };
//         let m = Message::Version(p);
//         m.write(&mut v, magic).unwrap();
//         assert!(Message::read(&mut Cursor::new(&v), magic).unwrap() == m);
//     }
//
//     #[test]
//     #[should_panic]
//     fn write_other_errors() {
//         let mut v = Vec::new();
//         let m = Message::Other("Unknown message".to_string());
//         m.write(&mut v, [7, 8, 9, 0]).unwrap();
//     }
//
//     #[test]
//     fn read_other() {
//         let magic = [7, 8, 9, 0];
//         let command = *b"unknowncmd\0\0";
//         let header = MessageHeader {
//             magic,
//             command,
//             payload_size: 0,
//             checksum: NO_CHECKSUM,
//         };
//         let mut v = Vec::new();
//         header.write(&mut v).unwrap();
//         let mut cursor = Cursor::new(&v);
//         let m = Message::read(&mut cursor, magic).unwrap();
//         if let Message::Other(_) = m {
//             // Success
//         } else {
//             assert!(false);
//         }
//     }
// }
