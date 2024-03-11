use std::sync::Arc;
use clap::Parser;
use env_logger::Env;
use bitcoinsv::bitcoin::BlockchainId;
use bitcoinsv::p2p::{Connection, ConnectionConfig, PeerAddress};
use log::info;


/// A simple example of connecting to a P2P peer and displaying all messages from that peer.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The ip address of the peer to connect to.
    #[clap(index=1)]
    ip: String,
    /// The port of the peer to connect to.
    #[clap(long, default_value = "8333")]
    port: u16,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args: Args = Args::parse();
    let peer = PeerAddress::new(format!("{}:{}", args.ip, args.port).parse().unwrap());
    let config = Arc::new(ConnectionConfig::default_for(BlockchainId::Mainnet));
    let (c, handle) = Connection::new(peer, config, None);
    let mut rx = c.subscribe();
    loop {
        match rx.recv().await {
            Ok(envelope) => {
                info!("{}", envelope.message);
            }
            Err(_e) => {
                break;
            }
        }
    }
    c.close().await;
    handle.await.unwrap();
}
