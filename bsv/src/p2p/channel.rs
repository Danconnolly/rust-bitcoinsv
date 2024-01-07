use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use crate::p2p::ACTOR_CHANNEL_SIZE;
use crate::p2p::connection::{Connection, ConnectionConfig};
use crate::p2p::messages::P2PMessageChannelSender;

/// A PeerChannel is a single TCP connection to a peer.
/// A peer channel is complete in the sense that it can send and receive any type of message. The
/// higher-level Connection is responsible for prioritizing messages between different peer channels.
pub struct PeerChannel {
    sender: Sender<ChannelControlMessage>,
}

impl PeerChannel {
    pub fn new(config: Arc<ConnectionConfig>, msg_channel: Option<P2PMessageChannelSender>) -> (Self, JoinHandle<()>) {
        let (tx, rx) = channel(ACTOR_CHANNEL_SIZE);
        let j = tokio::spawn(async move { PeerChannelActor::new(rx, config, msg_channel).await });
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
    config: Arc<ConnectionConfig>,
    msg_channel: Option<P2PMessageChannelSender>,
}

impl PeerChannelActor {
    async fn new(receiver: Receiver<ChannelControlMessage>, config: Arc<ConnectionConfig>, msg_channel: Option<P2PMessageChannelSender>) {
        let mut p = PeerChannelActor { inbox: receiver, config, msg_channel };
        p.main().await;
    }

    async fn main(&mut self) {
        loop {
            tokio::select! {
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
}
