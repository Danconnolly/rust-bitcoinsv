use std::collections::VecDeque;
use std::sync::Arc;
use log::{trace, warn};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::io::AsyncReadExt;
use crate::Result;
use crate::p2p::{ACTOR_CHANNEL_SIZE, PeerAddress};
use crate::p2p::connection::GlobalConnectionConfig;
use crate::p2p::messages::{P2PMessage, P2PMessageChannelSender, DEFAULT_MAX_PAYLOAD_SIZE, Version, P2PMessageType};
use crate::p2p::params::NetworkParams;

/// A PeerChannel is a single TCP connection to a peer.
///
/// The PeerChannel only handles sending and receiving messages. The higher level Connection
/// handles either dealing with the messages or handing the message off.
///
/// A peer channel is complete in the sense that it can send and receive any type of message. The
/// higher-level Connection is responsible for prioritizing messages between different peer channels.
pub struct PeerChannel {
    sender: Sender<ChannelControlMessage>,
}

impl PeerChannel {
    pub fn new(address: PeerAddress, config: Arc<GlobalConnectionConfig>, network_params: NetworkParams, msg_channel: Option<P2PMessageChannelSender>) -> (Self, JoinHandle<()>) {
        let (tx, rx) = channel(ACTOR_CHANNEL_SIZE);
        let j = tokio::spawn(async move { PeerChannelActor::new(rx, address, config, network_params, msg_channel).await });
        (PeerChannel { sender: tx }, j)
    }

    pub async fn close(&self) {
        self.sender.send(ChannelControlMessage::Close).await.unwrap();
    }
}

pub enum ChannelControlMessage {
    Close,
}


struct PeerChannelActor {
    inbox: Receiver<ChannelControlMessage>,
    peer: PeerAddress,
    config: Arc<GlobalConnectionConfig>,
    network_params: NetworkParams,
    msg_channel: Option<P2PMessageChannelSender>,
    send_queue: VecDeque<P2PMessage>,
    writing: bool,
    version_received: bool,
}

impl PeerChannelActor {
    async fn new(receiver: Receiver<ChannelControlMessage>, peer_address: PeerAddress, config: Arc<GlobalConnectionConfig>, network_params: NetworkParams,
                 msg_channel: Option<P2PMessageChannelSender>) {
        let mut p = PeerChannelActor {
            inbox: receiver, peer: peer_address, config, network_params, msg_channel,
            send_queue: Default::default(),
            writing: false,
            version_received: false,
        };
        p.main().await;
    }

    async fn main(&mut self) {
        trace!("PeerChannelActor started.");
        let mut stream = TcpStream::connect(self.peer.address.clone()).await.unwrap();
        trace!("PeerChannelActor connected to {:?}", self.peer);
        let (mut reader, mut writer) = stream.split();
        let v = Version::default();
        let v_msg = P2PMessage::Version(v);
        let _ = v_msg.write(&mut writer, self.network_params.magic).await.unwrap();
        loop {
            tokio::select! {
                msg = P2PMessage::read(&mut reader, self.network_params.magic, DEFAULT_MAX_PAYLOAD_SIZE) => {
                    self.handle_received(&msg).await;
                },
                Some(msg) = self.inbox.recv() => {
                    match msg {
                        ChannelControlMessage::Close => {
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Handle the received P2P Message
    async fn handle_received(&mut self, msg_result: &Result<P2PMessage>) {
        match msg_result {
            Ok(msg) => {
                trace!("msg received: {:?}", msg);
                // match P2PMessageType::from(msg) {
                //     P2PMessageType::Data => {
                //         match self.msg_channel {
                //             Some(sender) => sender.send(msg.clone()),
                //             None => {}
                //         }
                //     }
                //     P2PMessageType::ConnectionControl => {}
                // }
            },
            Err(e) => {
                // todo: handle various types of errors
                warn!("error receiving message from peer, connection: {}, peer: {}, error: {}", 0, self.peer.id, e);
            }
        }
    }

    fn send_msg(&mut self, msg: P2PMessage) {
        self.send_queue.push_back(msg);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use tokio::time::sleep;
    use super::*;
    use crate::bitcoin::BlockchainId::Mainnet;

    // todo: get some tests where it is talking to itself once a listener has been implemented
    
    // #[tokio::test]
    // async fn start_stop_test() {
    //     let address = PeerAddress::new("127.0.0.1:8333".parse().unwrap());
    //     let (h, j) = PeerChannel::new(address, Arc::new(GlobalConnectionConfig::default(Mainnet)), NetworkParams::from(Mainnet), None);
    //     let _ = sleep(Duration::from_secs(10)).await;
    //     h.close().await;
    //     j.await.expect("Channel failed");
    // }
}
