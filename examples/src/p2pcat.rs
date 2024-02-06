use std::sync::Arc;
use bitcoinsv::bitcoin::BlockchainId;
use bitcoinsv::p2p::{Connection, ConnectionConfig, PeerAddress};

/// This is a simple example of connecting to a P2P peer and displaying all messages from that peer.
#[tokio::main]
async fn main() {
    env_logger::init();
    let peer = PeerAddress::new("95.216.243.249:8333".parse().unwrap());
    let config = Arc::new(ConnectionConfig::default_for(BlockchainId::Mainnet));
    let (c, handle) = Connection::new(peer, config, None);
    let mut rx = c.subscribe();
    loop {
        match rx.recv().await {
            Ok(envelope) => {
                println!("{:?}", envelope.message);
            }
            Err(_e) => {
                break;
            }
        }
    }
    c.close().await;
    handle.await.unwrap();
}
