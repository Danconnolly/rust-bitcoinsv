use std::thread::sleep;
use clap::Parser;
use env_logger::Env;
use bitcoinsv::bitcoin::BlockchainId::Mainnet;
use bitcoinsv::bitcoin::FromHex;
use bitcoinsv::bitcoin::hash::Hash;
use bitcoinsv::p2p::{P2PManager, P2PManagerConfig};

/// Retrieves a block from the network.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The hash of the block to retrieve.
    #[clap(index=1)]
    hash: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args: Args = Args::parse();
    let block_hash = Hash::from_hex(args.hash).unwrap();
    let p2p_mgr_config = P2PManagerConfig::default(Mainnet);
    let (p2p_manager, mgr_handle) = P2PManager::new(p2p_mgr_config);

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    p2p_manager.stop().await.expect("couldn't stop p2pmanager");
    mgr_handle.await.expect("p2pmanager didnt stop");
}
