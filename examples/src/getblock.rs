use std::sync::Arc;
use clap::Parser;
use env_logger::Env;
use bitcoinsv::bitcoin::BlockchainId::Main;
use bitcoinsv::bitcoin::{BlockchainId, FromHex, Hash};
use bitcoinsv::p2p::{Connection, ConnectionConfig, P2PManager, P2PManagerConfig, PeerAddress};

/// Retrieves a block from the network.
///
/// THIS DOES NOT WORK YET
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The ip address of the peer to connect to.
    #[clap(index=1)]
    ip: String,
    /// The port of the peer to connect to.
    #[clap(long, default_value = "8333")]
    port: u16,
    /// The hash of the block to retrieve.
    #[clap(index=2)]
    hash: String,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args: Args = Args::parse();
    let peer = PeerAddress::new(format!("{}:{}", args.ip, args.port).parse().unwrap());
    let block_hash = Hash::from_hex(args.hash).unwrap();
    let config = Arc::new(ConnectionConfig::default_for(BlockchainId::Main));
    let (c, handle) = Connection::new(peer, config, None);

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    c.close().await;
    handle.await.unwrap();
}
