use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::bitcoin::Encodable;
use crate::util::epoch_secs_u32;

// based on code imported from rust-sv
// this struct includes the timestamp and is therefore not suitable for direct use in the version message
// https://en.bitcoin.it/wiki/Protocol_documentation#Network_address

/// Network address for a node on the network
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct NodeAddr {
    /// Timestamp of the address
    pub timestamp: u32,
    /// Services flags for the node
    pub services: u64,
    /// IP address for the node
    pub ip: IpAddr,
    /// Port for Bitcoin P2P communication
    pub port: u16,
}

impl NodeAddr {
    /// Size of the NodeAddr in bytes
    pub const SIZE: usize = 30;

    /// Creates a NodeAddr from an IP address and port
    pub fn new(ip: IpAddr, port: u16) -> NodeAddr {
        NodeAddr {
            timestamp: epoch_secs_u32(),
            services: 0,
            ip,
            port,
        }
    }
}

impl Default for NodeAddr {
    fn default() -> NodeAddr {
        NodeAddr {
            timestamp: epoch_secs_u32(),
            services: 0,
            ip: IpAddr::from([0; 16]),
            port: 0,
        }
    }
}

impl Encodable for NodeAddr {
    fn decode<R: ReadBytesExt + Send>(reader: &mut R) -> crate::Result<Self> where Self: Sized {
        let timestamp = reader.read_u32::<LittleEndian>()?;
        let services = reader.read_u64::<LittleEndian>()?;
        let mut ip_bin = [0u8; 16];
        let _bytes_read = reader.read_exact(&mut ip_bin)?;    // big endian order
        let ip;
        if ip_bin[0..12] == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255] {
            // ipv4 mapped ipv6 address
            ip = IpAddr::V4(Ipv4Addr::from([ip_bin[12], ip_bin[13], ip_bin[14], ip_bin[15]]));
        } else {
            ip = IpAddr::V6(Ipv6Addr::from(ip_bin));
        }
        let port = reader.read_u16::<BigEndian>()?;        // big endian order
        Ok(NodeAddr { timestamp, services, ip, port, })
    }

    fn encode_into<W: WriteBytesExt + Send>(&self, writer: &mut W) -> crate::Result<()> {
        writer.write_u32::<LittleEndian>(self.timestamp)?;
        writer.write_u64::<LittleEndian>(self.services)?;
        match self.ip {
            IpAddr::V4(v4) => {
                writer.write_all(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255])?;
                writer.write_all(&v4.octets())?;
            },
            IpAddr::V6(v6) => {
                writer.write_all(&v6.octets())?;
            },
        }
        writer.write_u16::<BigEndian>(self.port)?;
        Ok(())
    }

    fn size(&self) -> usize {
        NodeAddr::SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;
    use std::io::Cursor;
    use std::net::Ipv4Addr;

    #[test]
    fn read_bytes() {
        let b =
            hex::decode(format!("{}{}{}{}",
                                "5F849A65",         // timestamp = 1_704_625_247, hex = 65 9A 84 5F, little endian = 5F 84 9A 65
                                "2500000000000000", // services = 37, hex = 25, little endian = 25 00 00 00 00 00 00 00
                                "00000000000000000000ffff2d32bffb", // ip = 45.50.191.251, hex = 2d32bffb, ipv6 mapped = 0000:0000:0000:0000:0000:ffff:2d32:bffb
                                "ddd3")             // port = 56787
                                .as_bytes()).unwrap();
        let a = NodeAddr::decode(&mut Cursor::new(&b)).unwrap();
        assert_eq!(a.timestamp, 1_704_625_247);
        assert_eq!(a.services, 37);
        assert_eq!(a.ip, "45.50.191.251".parse::<Ipv4Addr>().unwrap());
        assert_eq!(a.port, 56787);
    }

    #[test]
    fn write_read() {
        let mut v = Vec::new();
        let a = NodeAddr {
            timestamp: 1_704_625_247,
            services: 1,
            ip: IpAddr::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
            port: 123,
        };
        a.encode_into(&mut v).unwrap();
        assert_eq!(v.len(), NodeAddr::SIZE);
        assert_eq!(NodeAddr::decode(&mut Cursor::new(&v)).unwrap(), a);
    }
}