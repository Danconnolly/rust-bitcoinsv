use crate::bitcoin::{BlockHeader, Hash, Tx};
use crate::{Error, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Magic values for different networks
pub const MAGIC_MAINNET: u32 = 0xE3E1F3E8;
pub const MAGIC_TESTNET: u32 = 0xF4E5F3F4;
pub const MAGIC_REGTEST: u32 = 0xDAB5BFFA;

/// Maximum payload size (32MB)
pub const MAX_PAYLOAD_SIZE: u32 = 32 * 1024 * 1024;

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 70015;

/// Services bitfield
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Services(pub u64);

impl Services {
    /// No services
    pub const NONE: Services = Services(0);
    /// NODE_NETWORK - full node
    pub const NETWORK: Services = Services(1);
    /// NODE_GETUTXO
    pub const GETUTXO: Services = Services(2);
    /// NODE_BLOOM - BIP 37
    pub const BLOOM: Services = Services(4);
    /// NODE_WITNESS - SegWit
    pub const WITNESS: Services = Services(8);
    /// NODE_NETWORK_LIMITED - pruned node
    pub const NETWORK_LIMITED: Services = Services(1024);
}

/// Network address with timestamp and services
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkAddress {
    pub timestamp: Option<u32>, // Only used in addr messages
    pub services: Services,
    pub addr: IpAddr,
    pub port: u16,
}

impl NetworkAddress {
    /// Encode the address (without timestamp)
    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u64_le(self.services.0);

        // Encode IP address as 16 bytes (IPv4-mapped IPv6)
        match self.addr {
            IpAddr::V4(addr) => {
                // IPv4-mapped IPv6 prefix
                buf.put_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF]);
                buf.put_slice(&addr.octets());
            }
            IpAddr::V6(addr) => {
                buf.put_slice(&addr.octets());
            }
        }

        buf.put_u16(self.port.to_be()); // Port is big-endian
    }

    /// Decode the address (without timestamp)
    pub fn decode(buf: &mut dyn Buf) -> Result<Self> {
        if buf.remaining() < 26 {
            return Err(Error::BadData(
                "Insufficient data for network address".to_string(),
            ));
        }

        let services = Services(buf.get_u64_le());

        // Read 16-byte IP address
        let mut ip_bytes = [0u8; 16];
        buf.copy_to_slice(&mut ip_bytes);

        // Check if it's an IPv4-mapped IPv6 address
        let addr = if ip_bytes[0..12] == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF] {
            IpAddr::V4(Ipv4Addr::new(
                ip_bytes[12],
                ip_bytes[13],
                ip_bytes[14],
                ip_bytes[15],
            ))
        } else {
            IpAddr::V6(Ipv6Addr::from(ip_bytes))
        };

        let port = buf.get_u16().to_be(); // Port is big-endian

        Ok(NetworkAddress {
            timestamp: None,
            services,
            addr,
            port,
        })
    }
}

/// Message header
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageHeader {
    pub magic: u32,
    pub command: [u8; 12],
    pub payload_size: u32,
    pub checksum: u32,
}

impl MessageHeader {
    pub const SIZE: usize = 24;

    /// Create a new message header
    pub fn new(magic: u32, command: &str, payload: &[u8]) -> Self {
        let mut cmd_bytes = [0u8; 12];
        let cmd_slice = command.as_bytes();
        cmd_bytes[..cmd_slice.len().min(12)].copy_from_slice(&cmd_slice[..cmd_slice.len().min(12)]);

        // Calculate checksum (first 4 bytes of double SHA256)
        let hash = Hash::sha256d(payload);
        let checksum = u32::from_le_bytes([hash.raw[0], hash.raw[1], hash.raw[2], hash.raw[3]]);

        MessageHeader {
            magic,
            command: cmd_bytes,
            payload_size: payload.len() as u32,
            checksum,
        }
    }

    /// Get command as string
    pub fn command_string(&self) -> String {
        let end = self.command.iter().position(|&b| b == 0).unwrap_or(12);
        String::from_utf8_lossy(&self.command[..end]).to_string()
    }

    /// Encode the header
    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u32_le(self.magic);
        buf.put_slice(&self.command);
        buf.put_u32_le(self.payload_size);
        buf.put_u32_le(self.checksum);
    }

    /// Decode the header
    pub fn decode(buf: &mut dyn Buf) -> Result<Self> {
        if buf.remaining() < Self::SIZE {
            return Err(Error::BadData(
                "Insufficient data for message header".to_string(),
            ));
        }

        let magic = buf.get_u32_le();
        let mut command = [0u8; 12];
        buf.copy_to_slice(&mut command);
        let payload_size = buf.get_u32_le();
        let checksum = buf.get_u32_le();

        if payload_size > MAX_PAYLOAD_SIZE {
            return Err(Error::BadData(format!(
                "Payload size {} exceeds maximum",
                payload_size
            )));
        }

        Ok(MessageHeader {
            magic,
            command,
            payload_size,
            checksum,
        })
    }
}

/// Message types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    // Control messages
    Version(VersionMessage),
    Verack,
    Ping(u64),
    Pong(u64),

    // Address messages
    Addr(Vec<NetworkAddress>),
    GetAddr,

    // Inventory messages
    Inv(Vec<Inventory>),
    GetData(Vec<Inventory>),
    NotFound(Vec<Inventory>),

    // Block messages
    Block(Bytes), // Placeholder - in real implementation would be Block
    GetBlocks(GetBlocksMessage),
    GetHeaders(GetBlocksMessage),
    Headers(Vec<BlockHeader>),

    // Transaction messages
    Tx(Tx),
    GetTx(Hash),

    // Other messages
    Reject(RejectMessage),
    SendHeaders,
    FeeFilter(u64),
}

impl Message {
    /// Get the command string for this message
    pub fn command(&self) -> &'static str {
        match self {
            Message::Version(_) => "version",
            Message::Verack => "verack",
            Message::Ping(_) => "ping",
            Message::Pong(_) => "pong",
            Message::Addr(_) => "addr",
            Message::GetAddr => "getaddr",
            Message::Inv(_) => "inv",
            Message::GetData(_) => "getdata",
            Message::NotFound(_) => "notfound",
            Message::Block(_) => "block",
            Message::GetBlocks(_) => "getblocks",
            Message::GetHeaders(_) => "getheaders",
            Message::Headers(_) => "headers",
            Message::Tx(_) => "tx",
            Message::GetTx(_) => "gettx",
            Message::Reject(_) => "reject",
            Message::SendHeaders => "sendheaders",
            Message::FeeFilter(_) => "feefilter",
        }
    }

    /// Encode the message payload
    pub fn encode_payload(&self, buf: &mut BytesMut) -> Result<()> {
        match self {
            Message::Version(msg) => msg.encode(buf),
            Message::Verack => Ok(()),
            Message::Ping(nonce) => {
                buf.put_u64_le(*nonce);
                Ok(())
            }
            Message::Pong(nonce) => {
                buf.put_u64_le(*nonce);
                Ok(())
            }
            Message::Addr(addrs) => {
                encode_var_int(addrs.len() as u64, buf);
                for addr in addrs {
                    if let Some(timestamp) = addr.timestamp {
                        buf.put_u32_le(timestamp);
                    }
                    addr.encode(buf);
                }
                Ok(())
            }
            Message::GetAddr => Ok(()),
            Message::Inv(items) => encode_inventory_vector(items, buf),
            Message::GetData(items) => encode_inventory_vector(items, buf),
            Message::NotFound(items) => encode_inventory_vector(items, buf),
            Message::Block(block_data) => {
                buf.put_slice(block_data);
                Ok(())
            }
            Message::GetBlocks(msg) => msg.encode(buf),
            Message::GetHeaders(msg) => msg.encode(buf),
            Message::Headers(headers) => {
                encode_var_int(headers.len() as u64, buf);
                for _header in headers {
                    // In real implementation, would encode each header
                    buf.put_slice(&[0u8; 80]); // Placeholder
                    encode_var_int(0, buf); // tx count (always 0 for headers)
                }
                Ok(())
            }
            Message::Tx(_tx) => {
                // For now, just put a placeholder
                // In real implementation, would use tx.encode()
                buf.put_slice(b"tx_data");
                Ok(())
            }
            Message::GetTx(hash) => {
                buf.put_slice(&hash.raw);
                Ok(())
            }
            Message::Reject(msg) => msg.encode(buf),
            Message::SendHeaders => Ok(()),
            Message::FeeFilter(fee_rate) => {
                buf.put_u64_le(*fee_rate);
                Ok(())
            }
        }
    }
}

/// Version message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionMessage {
    pub version: u32,
    pub services: Services,
    pub timestamp: i64,
    pub recv_addr: NetworkAddress,
    pub from_addr: NetworkAddress,
    pub nonce: u64,
    pub user_agent: String,
    pub start_height: u32,
    pub relay: bool,
}

impl VersionMessage {
    pub fn encode(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u32_le(self.version);
        buf.put_u64_le(self.services.0);
        buf.put_i64_le(self.timestamp);
        self.recv_addr.encode(buf);
        self.from_addr.encode(buf);
        buf.put_u64_le(self.nonce);

        // Encode user agent as var_string
        let ua_bytes = self.user_agent.as_bytes();
        encode_var_int(ua_bytes.len() as u64, buf);
        buf.put_slice(ua_bytes);

        buf.put_u32_le(self.start_height);
        buf.put_u8(if self.relay { 1 } else { 0 });

        Ok(())
    }
}

/// Inventory item type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvType {
    Error = 0,
    Tx = 1,
    Block = 2,
    FilteredBlock = 3,
    CompactBlock = 4,
}

/// Inventory item
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    pub inv_type: InvType,
    pub hash: Hash,
}

/// GetBlocks/GetHeaders message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetBlocksMessage {
    pub version: u32,
    pub locator_hashes: Vec<Hash>,
    pub hash_stop: Hash,
}

impl GetBlocksMessage {
    pub fn encode(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u32_le(self.version);
        encode_var_int(self.locator_hashes.len() as u64, buf);
        for hash in &self.locator_hashes {
            buf.put_slice(&hash.raw);
        }
        buf.put_slice(&self.hash_stop.raw);
        Ok(())
    }
}

/// Reject message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectMessage {
    pub message: String,
    pub code: u8,
    pub reason: String,
    pub data: Option<Bytes>,
}

impl RejectMessage {
    pub fn encode(&self, buf: &mut BytesMut) -> Result<()> {
        // Encode message
        let msg_bytes = self.message.as_bytes();
        encode_var_int(msg_bytes.len() as u64, buf);
        buf.put_slice(msg_bytes);

        // Encode code
        buf.put_u8(self.code);

        // Encode reason
        let reason_bytes = self.reason.as_bytes();
        encode_var_int(reason_bytes.len() as u64, buf);
        buf.put_slice(reason_bytes);

        // Encode optional data
        if let Some(data) = &self.data {
            buf.put_slice(data);
        }

        Ok(())
    }
}

/// Encode a variable-length integer
fn encode_var_int(n: u64, buf: &mut BytesMut) {
    if n < 0xFD {
        buf.put_u8(n as u8);
    } else if n <= 0xFFFF {
        buf.put_u8(0xFD);
        buf.put_u16_le(n as u16);
    } else if n <= 0xFFFFFFFF {
        buf.put_u8(0xFE);
        buf.put_u32_le(n as u32);
    } else {
        buf.put_u8(0xFF);
        buf.put_u64_le(n);
    }
}

/// Encode an inventory vector
fn encode_inventory_vector(items: &[Inventory], buf: &mut BytesMut) -> Result<()> {
    encode_var_int(items.len() as u64, buf);
    for item in items {
        buf.put_u32_le(item.inv_type as u32);
        buf.put_slice(&item.hash.raw);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex::FromHex;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_message_header_encode_decode() {
        let payload = b"test payload";
        let header = MessageHeader::new(MAGIC_MAINNET, "version", payload);

        let mut buf = BytesMut::new();
        header.encode(&mut buf);

        assert_eq!(buf.len(), MessageHeader::SIZE);

        let mut buf = buf.freeze();
        let decoded = MessageHeader::decode(&mut buf).unwrap();

        assert_eq!(header.magic, decoded.magic);
        assert_eq!(header.command, decoded.command);
        assert_eq!(header.payload_size, decoded.payload_size);
        assert_eq!(header.checksum, decoded.checksum);
        assert_eq!(header.command_string(), "version");
    }

    #[test]
    fn test_network_address_encode_decode() {
        // Test IPv4 address
        let addr = NetworkAddress {
            timestamp: None,
            services: Services::NETWORK,
            addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            port: 8333,
        };

        let mut buf = BytesMut::new();
        addr.encode(&mut buf);

        let mut buf = buf.freeze();
        let decoded = NetworkAddress::decode(&mut buf).unwrap();

        assert_eq!(addr.services, decoded.services);
        assert_eq!(addr.addr, decoded.addr);
        assert_eq!(addr.port, decoded.port);

        // Test IPv6 address
        let addr_v6 = NetworkAddress {
            timestamp: None,
            services: Services::NETWORK,
            addr: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            port: 8333,
        };

        let mut buf = BytesMut::new();
        addr_v6.encode(&mut buf);

        let mut buf = buf.freeze();
        let decoded_v6 = NetworkAddress::decode(&mut buf).unwrap();

        assert_eq!(addr_v6.services, decoded_v6.services);
        assert_eq!(addr_v6.addr, decoded_v6.addr);
        assert_eq!(addr_v6.port, decoded_v6.port);
    }

    #[test]
    fn test_version_message_encode() {
        let version_msg = VersionMessage {
            version: PROTOCOL_VERSION,
            services: Services::NETWORK,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            recv_addr: NetworkAddress {
                timestamp: None,
                services: Services::NETWORK,
                addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                port: 8333,
            },
            from_addr: NetworkAddress {
                timestamp: None,
                services: Services::NETWORK,
                addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                port: 8334,
            },
            nonce: 0x1234567890ABCDEF,
            user_agent: "/rust-bitcoinsv:0.1.0/".to_string(),
            start_height: 700000,
            relay: true,
        };

        let mut buf = BytesMut::new();
        version_msg.encode(&mut buf).unwrap();

        // Basic checks
        assert!(buf.len() > 80); // Should be at least this long

        // Check version at start
        let version = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(version, PROTOCOL_VERSION);
    }

    #[test]
    fn test_ping_pong_messages() {
        let nonce = 0x1234567890ABCDEF;

        // Test ping
        let ping = Message::Ping(nonce);
        assert_eq!(ping.command(), "ping");

        let mut buf = BytesMut::new();
        ping.encode_payload(&mut buf).unwrap();
        assert_eq!(buf.len(), 8);

        let encoded_nonce = u64::from_le_bytes([
            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        ]);
        assert_eq!(encoded_nonce, nonce);

        // Test pong
        let pong = Message::Pong(nonce);
        assert_eq!(pong.command(), "pong");

        let mut buf = BytesMut::new();
        pong.encode_payload(&mut buf).unwrap();
        assert_eq!(buf.len(), 8);
    }

    #[test]
    fn test_inventory_encoding() {
        let inv_items = vec![
            Inventory {
                inv_type: InvType::Tx,
                hash: Hash::from_hex(
                    "0000000000000000000000000000000000000000000000000000000000000001",
                )
                .unwrap(),
            },
            Inventory {
                inv_type: InvType::Block,
                hash: Hash::from_hex(
                    "0000000000000000000000000000000000000000000000000000000000000002",
                )
                .unwrap(),
            },
        ];

        let inv_msg = Message::Inv(inv_items.clone());
        assert_eq!(inv_msg.command(), "inv");

        let mut buf = BytesMut::new();
        inv_msg.encode_payload(&mut buf).unwrap();

        // Check var_int for count (2)
        assert_eq!(buf[0], 2);

        // Check first item type
        assert_eq!(buf[1], InvType::Tx as u8);

        // Check second item type (after first hash)
        assert_eq!(buf[1 + 4 + 32], InvType::Block as u8);
    }

    #[test]
    fn test_reject_message() {
        let reject = RejectMessage {
            message: "tx".to_string(),
            code: 0x10, // REJECT_INVALID
            reason: "bad-txns-inputs-spent".to_string(),
            data: Some(Bytes::from_static(b"extra_data")),
        };

        let reject_msg = Message::Reject(reject);
        assert_eq!(reject_msg.command(), "reject");

        let mut buf = BytesMut::new();
        reject_msg.encode_payload(&mut buf).unwrap();

        // Should contain the message, code, reason, and data
        assert!(buf.len() > 10);
    }

    #[test]
    fn test_varint_encoding() {
        let mut buf = BytesMut::new();

        // Test small values
        encode_var_int(10, &mut buf);
        assert_eq!(buf[0], 10);
        buf.clear();

        // Test 0xFD boundary
        encode_var_int(0xFC, &mut buf);
        assert_eq!(buf[0], 0xFC);
        buf.clear();

        encode_var_int(0xFD, &mut buf);
        assert_eq!(buf[0], 0xFD);
        assert_eq!(buf.len(), 3);
        buf.clear();

        // Test 0xFFFF boundary
        encode_var_int(0xFFFF, &mut buf);
        assert_eq!(buf[0], 0xFD);
        buf.clear();

        encode_var_int(0x10000, &mut buf);
        assert_eq!(buf[0], 0xFE);
        assert_eq!(buf.len(), 5);
        buf.clear();

        // Test large values
        encode_var_int(0x100000000, &mut buf);
        assert_eq!(buf[0], 0xFF);
        assert_eq!(buf.len(), 9);
    }

    #[test]
    fn test_services_flags() {
        assert_eq!(Services::NONE.0, 0);
        assert_eq!(Services::NETWORK.0, 1);
        assert_eq!(Services::GETUTXO.0, 2);
        assert_eq!(Services::BLOOM.0, 4);
        assert_eq!(Services::WITNESS.0, 8);
        assert_eq!(Services::NETWORK_LIMITED.0, 1024);

        // Test combining flags
        let combined = Services(Services::NETWORK.0 | Services::WITNESS.0);
        assert_eq!(combined.0, 9);
    }

    #[test]
    fn test_message_header_checksum() {
        let payload1 = b"test1";
        let payload2 = b"test2";

        let header1 = MessageHeader::new(MAGIC_MAINNET, "ping", payload1);
        let header2 = MessageHeader::new(MAGIC_MAINNET, "ping", payload2);

        // Different payloads should have different checksums
        assert_ne!(header1.checksum, header2.checksum);

        // Same payload should have same checksum
        let header3 = MessageHeader::new(MAGIC_MAINNET, "ping", payload1);
        assert_eq!(header1.checksum, header3.checksum);
    }

    #[test]
    fn test_command_string_null_termination() {
        let mut cmd_bytes = [0u8; 12];
        cmd_bytes[0..4].copy_from_slice(b"ping");

        let header = MessageHeader {
            magic: MAGIC_MAINNET,
            command: cmd_bytes,
            payload_size: 0,
            checksum: 0,
        };

        assert_eq!(header.command_string(), "ping");

        // Test with full 12 bytes
        let cmd_bytes_full = [b'a'; 12];
        let header_full = MessageHeader {
            magic: MAGIC_MAINNET,
            command: cmd_bytes_full,
            payload_size: 0,
            checksum: 0,
        };

        assert_eq!(header_full.command_string(), "aaaaaaaaaaaa");
    }

    #[test]
    fn test_getblocks_message() {
        let msg = GetBlocksMessage {
            version: PROTOCOL_VERSION,
            locator_hashes: vec![
                Hash::from_hex("0000000000000000000000000000000000000000000000000000000000000001")
                    .unwrap(),
                Hash::from_hex("0000000000000000000000000000000000000000000000000000000000000002")
                    .unwrap(),
            ],
            hash_stop: Hash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        };

        let mut buf = BytesMut::new();
        msg.encode(&mut buf).unwrap();

        // Check version
        let version = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(version, PROTOCOL_VERSION);

        // Check locator count
        assert_eq!(buf[4], 2); // var_int for 2 hashes

        // Total size: 4 (version) + 1 (count) + 64 (2 hashes) + 32 (stop hash)
        assert_eq!(buf.len(), 101);
    }

    #[test]
    fn test_addr_message_with_timestamps() {
        let addrs = vec![
            NetworkAddress {
                timestamp: Some(1234567890),
                services: Services::NETWORK,
                addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                port: 8333,
            },
            NetworkAddress {
                timestamp: Some(1234567891),
                services: Services::NETWORK,
                addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
                port: 8334,
            },
        ];

        let addr_msg = Message::Addr(addrs);
        assert_eq!(addr_msg.command(), "addr");

        let mut buf = BytesMut::new();
        addr_msg.encode_payload(&mut buf).unwrap();

        // Check count
        assert_eq!(buf[0], 2);

        // Each address: 4 (timestamp) + 8 (services) + 16 (ip) + 2 (port) = 30 bytes
        // Total: 1 (count) + 60 (2 addresses)
        assert_eq!(buf.len(), 61);
    }

    #[test]
    fn test_fee_filter_message() {
        let fee_rate = 1000u64; // satoshis per KB
        let msg = Message::FeeFilter(fee_rate);

        assert_eq!(msg.command(), "feefilter");

        let mut buf = BytesMut::new();
        msg.encode_payload(&mut buf).unwrap();

        assert_eq!(buf.len(), 8);
        let encoded_fee = u64::from_le_bytes([
            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        ]);
        assert_eq!(encoded_fee, fee_rate);
    }
}
