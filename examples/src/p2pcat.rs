
use bitcoinsv::bitcoin::BlockchainId;
use bitcoinsv::p2p::{P2PManager, P2PManagerConfig, PeerAddress};

/// This is a simple example of connecting to a P2P peer and displaying all messages from that peer.
#[tokio::main]
async fn main() {
    env_logger::init();
    let peer = PeerAddress::new("95.216.243.249:8333".parse().unwrap());
    let mut config = P2PManagerConfig::default(BlockchainId::Mainnet);
    config.add_peers = false;
    config.initial_peers.insert(0, peer);
    let (m, handle) = P2PManager::new(config);
    let mut rx = m.subscribe();
    loop {
        match rx.recv().await {
            Ok(msg) => {
                println!("{:?}", msg);
            }
            Err(_e) => {
                break;
            }
        }
    }
    m.stop().await;
    handle.await.unwrap();
}
