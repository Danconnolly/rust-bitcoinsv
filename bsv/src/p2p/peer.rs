use std::net::SocketAddr;
use uuid::Uuid;


/// A peer is an agent on the network to which a connection could be established.
pub struct Peer {
    id: Uuid,
    address: SocketAddr,
}

impl Peer {
    pub fn new(address: SocketAddr) -> Self {
        Self {
            id: Uuid::new_v4(),
            address,
        }
    }
}
