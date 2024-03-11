use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use async_trait::async_trait;
use log::warn;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::p2p::messages::node_addr::NodeAddr;
use crate::{Error, Result};
use crate::bitcoin::{Encodable, varint_decode, varint_encode, varint_size};
use crate::p2p::params::{MIN_SUPPORTED_PROTOCOL_VERSION, PROTOCOL_VERSION};
use crate::util::{epoch_secs, epoch_secs_u32};

/// Service flag that node is not a full node. Used for SPV wallets.
pub const NODE_NONE: u64 = 0;

/// Service flag that node is a full node and implements all protocol features
pub const NODE_NETWORK: u64 = 1;

/// Version payload defining a node's capabilities
/// todo: add support for message streams: https://github.com/bitcoin-sv-specs/protocol/blob/master/p2p/multistreams.md
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Version {
    /// The protocol version being used by the node.
    pub version: u32,
    /// Bitfield of features to be enabled for this connection.
    pub services: u64,
    /// Time since the Unix epoch in seconds.
    pub timestamp: i64,
    /// Network address of the node receiving this message.
    ///
    /// The timestamp field in this struct is ignored for Version messages.
    pub recv_addr: NodeAddr,
    /// Network address of the node emitting this message
    ///
    /// The timestamp field in this struct is ignored for Version messages.
    pub tx_addr: NodeAddr,
    /// A random nonce which can help a node detect a connection to itself.
    pub nonce: u64,
    /// User agent string
    pub user_agent: String,
    /// Height of the transmitting node's best block chain.
    pub start_height: i32,
    /// Whether the client wants to receive broadcast transactions before a filter is set.
    pub relay: bool,
}

impl Version {
    /// Checks if the version message is valid
    pub fn validate(&self) -> Result<()> {
        if self.version < MIN_SUPPORTED_PROTOCOL_VERSION {
            return Err(Error::BadData(format!("Unsupported protocol version: {}", self.version)));
        } else if self.version > PROTOCOL_VERSION {
            warn!("unknown protocol version: {}", self.version);
        }
        if (self.timestamp - epoch_secs()).abs() > 2 * 60 * 60 {
            return Err(Error::BadData(format!("Timestamp too old: {}", self.timestamp)));
        }
        Ok(())
    }

    // the version message does not include the timestamp in the addr, so we have our own function to read the
    // addr structure here
    async fn read_version_addr<R: AsyncReadExt + Unpin + Send>(reader: &mut R) -> Result<NodeAddr> where NodeAddr: Sized {
        let services = reader.read_u64_le().await?;
        let mut ip_bin = [0u8; 16];
        reader.read_exact(&mut ip_bin).await?;    // big endian order
        let ip= if ip_bin[0..12] == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255] {
            // ipv4 mapped ipv6 address
            IpAddr::V4(Ipv4Addr::from([ip_bin[12], ip_bin[13], ip_bin[14], ip_bin[15]]))
        } else {
            IpAddr::V6(Ipv6Addr::from(ip_bin))
        };
        let port = reader.read_u16().await?;        // big endian order
        Ok(NodeAddr { timestamp: epoch_secs_u32(), services, ip, port, })
    }

    // the version message does not include the timestamp in the addr, so we have our own function to write the
    // addr structure here
    async fn write_version_addr<W: AsyncWriteExt + Unpin + Send>(node_addr: &NodeAddr, writer: &mut W) -> Result<()> {
        writer.write_u64_le(node_addr.services).await?;
        match node_addr.ip {
            IpAddr::V4(v4) => {
                writer.write_all(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255]).await?;
                writer.write_all(&v4.octets()).await?;
            },
            IpAddr::V6(v6) => {
                writer.write_all(&v6.octets()).await?;
            },
        }
        writer.write_u16(node_addr.port).await?;            // big endian order
        Ok(())
    }
}

impl Default for Version {
    fn default() -> Self {
        Self {
            version: PROTOCOL_VERSION,
            services: NODE_NONE,
            timestamp: epoch_secs(),
            recv_addr: Default::default(),
            tx_addr: Default::default(),
            nonce: 0,
            user_agent: "rust-bitcoinsv".to_string(),
            start_height: 0,
            relay: true,
        }
    }
}

#[async_trait]
impl Encodable for Version {
    async fn decode_from<R: AsyncRead + Unpin + Send>(reader: &mut R) -> Result<Self> where Self: Sized {
        let version = reader.read_u32_le().await?;
        let services = reader.read_u64_le().await?;
        let timestamp = reader.read_i64_le().await?;
        let recv_addr = Version::read_version_addr(reader).await?;
        let tx_addr = Version::read_version_addr(reader).await?;
        let nonce = reader.read_u64_le().await?;
        let user_agent_size = varint_decode(reader).await?;
        // todo: check size before allocation
        let mut user_agent_bytes = vec![0; user_agent_size as usize];
        reader.read_exact(&mut user_agent_bytes).await?;
        let user_agent = String::from_utf8(user_agent_bytes)?;
        let start_height = reader.read_i32_le().await?;
        let relay = reader.read_u8().await? == 0x01;
        Ok(Version { version, services, timestamp, recv_addr, tx_addr, nonce, user_agent, start_height, relay, })
    }

    async fn encode_into<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32_le(self.version).await?;
        writer.write_u64_le(self.services).await?;
        writer.write_i64_le(self.timestamp).await?;
        Version::write_version_addr(&self.recv_addr, writer).await?;
        Version::write_version_addr(&self.tx_addr, writer).await?;
        writer.write_u64_le(self.nonce).await?;
        varint_encode(writer, self.user_agent.len() as u64).await?;
        writer.write_all(self.user_agent.as_bytes()).await?;
        writer.write_i32_le(self.start_height).await?;
        writer.write_u8(if self.relay { 0x01 } else { 0x00 }).await?;
        Ok(())
    }

    fn size(&self) -> usize {
        33 + (self.recv_addr.size() - 4)        // version addr is smaller
            + (self.tx_addr.size() - 4)
            + varint_size(self.user_agent.as_bytes().len() as u64)
            + self.user_agent.as_bytes().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    #[test]
    fn read_bytes() {
        let b = hex::decode("7f1101002500000000000000f2d2d25a00000000000000000000000000000000000000000000ffff2d32bffbdd1725000000000000000000000000000000000000000000000000008d501d3bb5369deb242f426974636f696e204142433a302e31362e30284542382e303b20626974636f7265292f6606080001".as_bytes()).unwrap();
        let v = Version::decode_from_buf(b.as_slice()).unwrap();
        assert_eq!(v.version, 70015);
        assert_eq!(v.services, 37);
        assert_eq!(v.timestamp, 1523766002);
        assert_eq!(v.recv_addr.services, 0);
        assert_eq!(v.recv_addr.ip, IpAddr::V4(Ipv4Addr::new(45, 50, 191, 251)));
        assert_eq!(v.recv_addr.port, 56599);
        assert_eq!(v.tx_addr.services, 37);
        assert_eq!(v.tx_addr.ip,  IpAddr::V6(Ipv6Addr::UNSPECIFIED));
        assert_eq!(v.tx_addr.port, 0);
        assert_eq!(v.nonce, 16977786322265395341);
        assert_eq!(v.user_agent, "/Bitcoin ABC:0.16.0(EB8.0; bitcore)/");
        assert_eq!(v.start_height, 525926);
        assert!(v.relay);
    }

    #[tokio::test]
    async fn write_read() {
        let m = Version {
            version: MIN_SUPPORTED_PROTOCOL_VERSION,
            services: 77,
            timestamp: 1234,
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
        let v = m.encode_into_buf().unwrap();
        assert_eq!(v.len(), m.size());
        assert_eq!(Version::decode_from_buf(v.as_slice()).unwrap(), m);
    }

    #[test]
    fn validate() {
        let m = Version {
            version: MIN_SUPPORTED_PROTOCOL_VERSION,
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
        // Valid
        assert!(m.validate().is_ok());
        // Unsupported version
        let m2 = Version {
            version: 0,
            ..m.clone()
        };
        assert!(m2.validate().is_err());
        // Bad timestamp
        let m3 = Version {
            timestamp: 0,
            ..m.clone()
        };
        assert!(m3.validate().is_err());
    }
}