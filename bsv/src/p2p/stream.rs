use std::sync::Arc;
use log::{info, trace, warn};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use crate::p2p::{ACTOR_CHANNEL_SIZE, PeerAddress};
use crate::p2p::config::CommsConfig;
use crate::p2p::connection::ConnectionConfig;
use crate::p2p::messages::{P2PMessage, P2PMessageChannelSender, Ping, Version, P2PMessageType};
use crate::p2p::messages::Protoconf;
use crate::p2p::params::DEFAULT_MAX_PAYLOAD_SIZE;

pub const P2P_COMMS_BUFFER_LENGTH: usize = 100;

// todo: implement support for protoconf, including inv limits

/// A PeerStream is a single TCP connection to a peer.
///
/// The PeerStream only handles sending and receiving messages. The higher level Connection
/// handles either dealing with the messages or handing the message off.
///
/// A peer stream is complete in the sense that it can send and receive any type of message. The
/// higher-level Connection is responsible for prioritizing messages between different peer streams.
pub struct PeerStream {
    sender: Sender<StreamControlMessage>,
}

impl PeerStream {
    pub fn new(address: PeerAddress, config: Arc<ConnectionConfig>, data_channel: P2PMessageChannelSender) -> (Self, JoinHandle<()>) {
        let (tx, rx) = channel(ACTOR_CHANNEL_SIZE);
        let j = tokio::spawn(async move { PeerStreamActor::new(rx, address, config, data_channel).await });
        (PeerStream { sender: tx }, j)
    }

    pub async fn close(&self) {
        self.sender.send(StreamControlMessage::Close).await.unwrap();
    }
}

pub enum StreamControlMessage {
    Close,
}

/// The state of the stream.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum StreamState {
    Starting,           // the stream is starting up
    Connecting,         // establishing TCP connection
    Handshaking,        // performing Bitcoin handshake
    Connected,          // connection fully established
    WaitForRetry,       // waiting for a retry
}

/// The stream actor.
struct PeerStreamActor {
    inbox: Receiver<StreamControlMessage>,             // control of the stream
    stream_state: StreamState,                        // current state of the stream
    peer: PeerAddress,
    conn_config: Arc<ConnectionConfig>,                 // the desired connection configuration
    config: Arc<RwLock<CommsConfig>>,                   // the active configuration for the stream
    data_channel: P2PMessageChannelSender,              // P2P Data messages are sent on this channel
    writer_rx: Option<Receiver<P2PMessage>>,
    writer_tx: Sender<P2PMessage>,
    reader_rx: Receiver<Arc<P2PMessage>>,
    reader_tx: Sender<Arc<P2PMessage>>,
    version_received: bool,                             // true if we have received a version message
    verack_received: bool,                              // true if we have received a verack message in response to our version
    send_headers: bool,                                 // has peer requested we send headers
    relay_tx: bool,                                     // has peer requested we relay transactions
}

impl PeerStreamActor {
    async fn new(receiver: Receiver<StreamControlMessage>, peer_address: PeerAddress, conn_config: Arc<ConnectionConfig>,
                 data_channel: P2PMessageChannelSender) {
        // prepare the channels, we will need these later
        let (reader_tx, reader_rx) = channel(P2P_COMMS_BUFFER_LENGTH);
        let (writer_tx, writer_rx) = channel(P2P_COMMS_BUFFER_LENGTH);
        let comms_config = CommsConfig::new(&conn_config);
        let mut p = PeerStreamActor {
            inbox: receiver, peer: peer_address,
            stream_state: StreamState::Starting,
            conn_config,
            config: Arc::new(RwLock::new(comms_config)),
            data_channel, writer_rx: Some(writer_rx), writer_tx, reader_rx, reader_tx,
            version_received: false,
            verack_received: false,
            send_headers: false,
            relay_tx: true,         // default is true, the peer can request not to relay tx
        };
        p.main().await;
    }

    async fn main(&mut self) {
        trace!("PeerStreamActor started.");
        self.stream_state = StreamState::Connecting;
        // todo: failure & retry logic
        let stream = TcpStream::connect(self.peer.address).await.unwrap();
        trace!("PeerChannelActor connected to {:?}", self.peer);
        let (reader, writer) = stream.into_split();
        let _r_handle = {
            // start the reader task
            let cfg = self.config.clone();
            let r_tx = self.reader_tx.clone();
            tokio::spawn(async move { PeerStreamActor::reader(r_tx, reader, cfg).await })
        };
        let _w_handle = {
            // start the writer task
            let cfg = self.config.clone();
            let w_rx = self.writer_rx.take().unwrap();
            tokio::spawn(async move { PeerStreamActor::writer(w_rx, writer, cfg).await })
        };
        self.stream_state = StreamState::Handshaking;
        // we send our version straightaway
        let v = Version::default();
        let v_msg = P2PMessage::Version(v);
        self.send_msg(v_msg).await;
        // the main loop
        loop {
            tokio::select! {
                Some(msg) = self.reader_rx.recv() => {
                    self.handle_received(msg.clone()).await;
                },
                Some(msg) = self.inbox.recv() => {
                    match msg {
                        StreamControlMessage::Close => {
                            break;
                        },
                    }
                }
            }
        }
    }

    /// Handle the received P2P Message
    async fn handle_received(&mut self, msg: Arc<P2PMessage>) {
        match self.stream_state {
            StreamState::Handshaking => {
                match &*msg {
                    P2PMessage::Version(v) => {
                        { let mut c = self.config.write().await;
                        c.protocol_version = v.version; }
                        self.relay_tx = v.relay;
                        let va = P2PMessage::Verack;
                        self.send_msg(va).await;
                        self.version_received = true;
                        trace!("received version message from peer: {}", self.peer.id);
                    }
                    P2PMessage::Verack => {
                        self.verack_received = true;
                        trace!("received verack message from peer: {}", self.peer.id);
                    }
                    _ => {
                        warn!("received unexpected message in handshaking state, message: {:?}", msg);
                    }
                };
                if self.version_received && self.verack_received {
                    info!("connected to peer: {}", self.peer.id);
                    self.stream_state = StreamState::Connected;
                    self.send_config().await;
                }
            },
            StreamState::Connected => {
                trace!("connected state msg received: {:?}", msg);
                match P2PMessageType::from(msg.clone()) {
                    P2PMessageType::Data => {
                        let _ = self.data_channel.send(msg);
                    }
                    P2PMessageType::ConnectionControl => {
                        match &*msg {
                            P2PMessage::Protoconf(p) => {
                                // we can send larger messages to the peer
                                let mut c = self.config.write().await;
                                c.max_send_payload_size = p.max_recv_payload_length as u64;
                            },
                            P2PMessage::SendHeaders => {
                                // we should send headers
                                self.send_headers = true;
                            },
                            P2PMessage::Ping(p) => {
                                let pong = Ping::new(p.nonce);
                                self.send_msg(P2PMessage::Pong(pong)).await;
                                trace!("sent pong message");
                            },
                            _ => {
                                warn!("received unexpected connection control message in connected state, message: {:?}", msg);
                            },
                        }
                        if self.conn_config.send_control_messages {
                            let _ = self.data_channel.send(msg.clone());
                        }
                    }
                }
            },
            _ => {
                warn!("received message in anomalous state, state: {:?}, peer: {}", self.stream_state, self.peer.id);
            },
        }
    }

    // Send a message to the peer
    async fn send_msg(&mut self, msg: P2PMessage) {
        let _ = self.writer_tx.send(msg).await;
    }

    // The writer task. It continually reads from the channel and writes to the socket.
    // It has not state, it just reads and writes what it is given. In particular, it does not check the message
    // size.
    async fn writer(mut rx: Receiver<P2PMessage>, mut writer: tokio::net::tcp::OwnedWriteHalf, shared_config: Arc<RwLock<CommsConfig>>) {
        trace!("writer task started.");
        loop {
            match rx.recv().await {
                Some(msg) => {
                    let config = shared_config.read().await.clone();
                    match msg.write(&mut writer, &config).await {
                        Ok(_) => {}
                        Err(e) => {
                            warn!("error writing message to peer, error: {}", e);
                        }
                    }
                }
                None => {
                    break;
                }
            }
        }
    }

    // The reader task. It continually reads from the socket and writes to the channel.
    // It has no state or intelligence, it just reads and writes.
    async fn reader(tx: Sender<Arc<P2PMessage>>, mut reader: tokio::net::tcp::OwnedReadHalf, shared_config: Arc<RwLock<CommsConfig>>) {
        trace!("reader task started.");
        loop {
            let config = shared_config.read().await.clone();
            match P2PMessage::read(&mut reader, &config).await {
                Ok(msg) => {
                    match tx.send(Arc::new(msg)).await {
                        Ok(_) => {}
                        Err(e) => {
                            warn!("channel reader: error sending message to tokio channel, error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("channel reader: error reading message from peer, error: {}", e);
                    break;
                }
            }
        }
    }

    // Send initial configuration messages after the handshake
    async fn send_config(&mut self) {
        // maybe send the protoconf message
        if self.conn_config.max_recv_payload_size > DEFAULT_MAX_PAYLOAD_SIZE && self.conn_config.max_recv_payload_size <= u32::MAX as u64 {
            let protoconf = Protoconf::new(self.conn_config.max_recv_payload_size as u32);
            let protoconf_msg = P2PMessage::Protoconf(protoconf);
            self.send_msg(protoconf_msg).await;
        }
        self.send_msg(P2PMessage::SendHeaders).await;
    }

}

#[cfg(test)]
mod tests {
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
