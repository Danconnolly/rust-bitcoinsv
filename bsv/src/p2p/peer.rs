use std::net::{IpAddr, SocketAddr};
use uuid::Uuid;


/// A PeerAddress is a potential agent on the network to which a connection could be established.
#[derive(Debug, Clone)]
pub struct PeerAddress {
    /// The unique identifier of the peer.
    ///
    /// This can be used to identify the peer in a database, for example.
    pub peer_id: Uuid,
    pub address: SocketAddr,
}

impl PeerAddress {
    /// Create a new peer with a random UUID.
    pub fn new(address: SocketAddr) -> Self {
        Self {
            peer_id: Uuid::new_v4(),
            address,
        }
    }

    pub fn ip(&self) -> IpAddr {
        self.address.ip()
    }
}
