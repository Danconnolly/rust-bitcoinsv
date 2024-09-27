use std::sync::Arc;
use log::{info, trace, warn};
use minactor::{create_actor, Actor, ActorRef, Control};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use crate::Result;
use crate::p2p::connection::ConnectionConfig;
use crate::p2p::envelope::{P2PEnvelope, P2PMessageChannelSender};
use crate::p2p::messages::{P2PMessage, Ping, Version, P2PMessageType};
use crate::p2p::messages::Protoconf;
use crate::p2p::params::{DEFAULT_MAX_PAYLOAD_SIZE, NetworkParams, PROTOCOL_VERSION};
use crate::p2p::PeerAddress;

pub const P2P_COMMS_BUFFER_LENGTH: usize = 100;

// todo: implement support for protoconf, including inv limits

/// ChannelConfig is the context for the communication across a single channel.
///
/// These parameters are used throughout the P2P protocol to determine message limits and other
/// communication patterns.
///
/// It can be derived from the [ConnectionConfig] but is specific to a single stream. Most of the
/// parameters are static and do not change during the lifetime of the stream, but there are a couple
/// that are determined during the extended handshake and will need to be updated.
///
/// It is expected that this struct will be a single instance that is potentially shared by several
/// threads (for example a reader and writer thread).
///
/// At the moment this is used by obtaining a clone using a read lock before every read and write but
/// this is inefficient and should be changed to a more efficient method. todo
#[derive(Debug, Clone)]
pub struct ChannelConfig {
    /// The identifier of the peer being connected to.
    pub peer_id: Uuid,
    /// The identifier of the connection.
    pub connection_id: Uuid,
    /// The identifier of the channel.
    pub channel_id: u16,
    /// Send control messages to data channel?
    pub send_control_messages: bool,
    /// The magic bytes used in the message header.
    pub magic: [u8; 4],
    /// The maximum payload size we want to receive, using protoconf.
    pub max_recv_payload_size: u64,
    /// The maximum payload size the peer wants to receive.
    pub max_send_payload_size: u64,
    /// The maximum size of a block that we will accept.
    pub excessive_block_size: u64,
    /// The protocol version used by the remote peer.
    pub protocol_version: u32,
}

impl ChannelConfig {
    pub fn new(config: &ConnectionConfig, peer_id: &Uuid, connection_id: &Uuid) -> ChannelConfig {
        let np = NetworkParams::from(config.blockchain);
        ChannelConfig {
            peer_id: peer_id.clone(), connection_id: connection_id.clone(), channel_id: 0,
            send_control_messages: config.send_control_messages, magic: np.magic.clone(),
            max_recv_payload_size: config.max_recv_payload_size,
            max_send_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,
            excessive_block_size: config.excessive_block_size,
            protocol_version: PROTOCOL_VERSION,
        }
    }
}

impl Default for ChannelConfig {
    fn default() -> Self {
        let connection_config = ConnectionConfig::default();
        ChannelConfig::new(&connection_config, &Uuid::new_v4(), &Uuid::new_v4())
    }
}

/// A PeerChannel is a single TCP connection to a peer.
///
/// The PeerChannel only handles sending and receiving messages. The higher level [Connection]
/// handles either dealing with the messages or handing the message off.
///
/// A PeerChannel is complete in the sense that it can send and receive any type of message. The
/// higher-level [Connection] is responsible for prioritizing messages between different channels.
pub struct PeerChannel {
    actor_ref: ActorRef<PeerChannelActor>,
}

impl PeerChannel {
    /// Create a new channel to a peer.
    pub async fn new(address: PeerAddress, config: Arc<RwLock<ChannelConfig>>, data_channel: P2PMessageChannelSender) -> Result<(Self, JoinHandle<()>)> {
        let actor = PeerChannelActor::new(address, config, data_channel);
        let (a_ref, j) = create_actor(actor).await?;
        Ok((PeerChannel { actor_ref: a_ref }, j))
    }

    pub async fn close(&self) {
        // todo: remove?
    }
}

#[derive(Debug, Clone)]
/// Messages for communicating with the [PeerChannelActor].
pub enum ChannelControlMessage {
    /// A message has been received from the peer. This is used internally and is sent from
    /// a reader task to the PeerChannelActor.
    PeerMsgReceived(Arc<P2PEnvelope>),
}

/// The state of the channel.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum ChannelState {
    /// the channel is starting up
    Starting,
    /// establishing TCP connection
    Connecting,
    /// performing Bitcoin handshake
    Handshaking,
    /// connection fully established
    Connected,
    /// connection is closed, waiting for a retry   // todo
    WaitForRetry,
    /// Connection is being closed
    Closing,
}

/// The channel actor. This does the work of establishing the TCP connection and translation
/// to and from internal structures to the P2P binary protocol.
struct PeerChannelActor {
    /// current state of the channel
    channel_state: ChannelState,
    peer: PeerAddress,
    /// the active configuration for the channel
    config: Arc<RwLock<ChannelConfig>>,
    /// P2P Data messages are sent to this tokio channel
    data_channel: P2PMessageChannelSender,
    /// Sender to writer task of messages to send.
    writer_tx: Option<Sender<P2PMessage>>,
    /// Handle to writer task.
    writer_handle: Option<JoinHandle<()>>,
    /// Handle to reader task.
    reader_handle: Option<JoinHandle<()>>,
    /// Cancellation token for sub-tasks (reader & writer)
    subtask_cancel: CancellationToken,
    /// true if we have received a version message
    version_received: bool,
    /// true if we have received a verack message in response to our version
    verack_received: bool,
    /// has peer requested we send headers?
    send_headers: bool,
    /// has peer requested we relay transactions?
    relay_tx: bool,
}

impl PeerChannelActor {
    /// Initialize a new [PeerChannelActor] struct, ready to be created as an actor.
    fn new(peer_address: PeerAddress, config: Arc<RwLock<ChannelConfig>>, data_channel: P2PMessageChannelSender) -> Self {
        PeerChannelActor {
            peer: peer_address, channel_state: ChannelState::Starting, config,
            data_channel, writer_tx: None, writer_handle: None,
            reader_handle: None, subtask_cancel: CancellationToken::new(),
            version_received: false,
            verack_received: false,
            send_headers: false,
            relay_tx: true,         // default is true, the peer can request not to relay tx
        }
    }

    /// Handle the received P2P Envelope.
    async fn handle_received(&mut self, envelope: Arc<P2PEnvelope>) {
        let msg = &envelope.message;
        match self.channel_state {
            ChannelState::Handshaking => {
                match msg {
                    P2PMessage::Version(v) => {
                        { let mut c = self.config.write().await;
                        c.protocol_version = v.version; }
                        self.relay_tx = v.relay;
                        let va = P2PMessage::Verack;
                        self.send_msg(va).await;
                        self.version_received = true;
                        trace!("received version message from peer: {}", self.peer.peer_id);
                    }
                    P2PMessage::Verack => {
                        self.verack_received = true;
                        trace!("received verack message from peer: {}", self.peer.peer_id);
                    }
                    _ => {
                        warn!("received unexpected message in handshaking state, message: {:?}", msg);
                    }
                };
                if self.version_received && self.verack_received {
                    info!("connected to peer: {}", self.peer.peer_id);
                    self.channel_state = ChannelState::Connected;
                    // todo: some sort of notification to owner?
                    self.send_config().await;
                }
            },
            ChannelState::Connected => {
                trace!("connected state msg received: {:?}", msg);
                match P2PMessageType::from(msg) {
                    P2PMessageType::Data => {
                        // todo: errors?
                        let _ = self.data_channel.send(envelope);
                    },
                    P2PMessageType::ConnectionControl => {
                        match msg {
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
                        if self.config.read().await.send_control_messages {
                            let _ = self.data_channel.send(envelope);
                        }
                    }
                }
            },
            _ => {
                warn!("received message in anomalous state, state: {:?}, peer: {}", self.channel_state, self.peer.peer_id);
            },
        }
    }

    /// Send a message to the peer.
    async fn send_msg(&mut self, msg: P2PMessage) {
        if let Some(writer_tx) = &mut self.writer_tx {
            if writer_tx.send(msg).await.is_err() {
                // todo: Handle send error
            }
        }
    }

    /// Send initial configuration messages after the handshake.
    async fn send_config(&mut self) {
        // send the protoconf message if necessary
        let max_recv_payload_size = self.config.read().await.max_recv_payload_size;
        if max_recv_payload_size > DEFAULT_MAX_PAYLOAD_SIZE && max_recv_payload_size <= u32::MAX as u64 {
            let protoconf = Protoconf::new(max_recv_payload_size as u32);
            let protoconf_msg = P2PMessage::Protoconf(protoconf);
            self.send_msg(protoconf_msg).await;
        }
        // always send SendHeaders
        self.send_msg(P2PMessage::SendHeaders).await;
    }

    /// The writer task. It reads [P2PMessage]s from the channel and writes them the socket.
    /// It has no state, it just reads and writes what it is given. In particular, it does not check
    /// the message size.
    /// This task is spawned by on_initialization().
    async fn writer(mut rx: Receiver<P2PMessage>, mut writer: tokio::net::tcp::OwnedWriteHalf,
                    shared_config: Arc<RwLock<ChannelConfig>>, cancel_token: CancellationToken) {
        trace!("writer task started.");
        loop {
            select! {
                _ = cancel_token.cancelled() => { break; }
                msg = rx.recv() => {
                    match msg {
                        Some(msg) => {
                            let config = shared_config.read().await.clone();
                            // respect the cancel token that arrives in middle of write
                            // writes could be long, either naturally or due to malicious actors
                            select! {
                                _ = cancel_token.cancelled() => { break; }
                                r = msg.write(&mut writer, &config) => {
                                    match r {
                                        Ok(_) => {}
                                        Err(e) => {
                                            warn!("error writing message to peer, error: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
        }
    }

    /// The reader task.
    ///
    /// It has no state or intelligence, it just reads messages from the socket and sends them
    /// to the actor as a [ChannelControlMessage::PeerMsgReceived].
    ///
    /// This task is spawned by on_initialization().
    async fn reader(actor: ActorRef<PeerChannelActor>, mut reader: tokio::net::tcp::OwnedReadHalf,
                    config: Arc<RwLock<ChannelConfig>>, cancel_token: CancellationToken) {
        trace!("reader task started.");
        loop {
            // todo: do we really need to clone it? doesnt that defeat the point?
            let config = config.read().await.clone();
            select! {
                _ = cancel_token.cancelled() => { break; }
                r = P2PMessage::read(&mut reader, &config) => {
                    match r {
                        Ok(msg) => {
                            let envelope = P2PEnvelope::new(msg, &config);
                            match actor.send(ChannelControlMessage::PeerMsgReceived(Arc::new(envelope))).await {
                                Ok(_) => {}
                                Err(e) => {
                                    warn!("stream reader: error sending message to actor, error: {:?}", e);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            warn!("stream reader: error reading message from peer, error: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    }
}

impl Actor for PeerChannelActor {
    type SendMessage = ChannelControlMessage;
    type CallMessage = ();
    type ErrorType = ();

    /// Called to initialize the actor.
    async fn on_initialization(&mut self, self_ref: ActorRef<Self>) -> Control {
        trace!("PeerStreamActor started.");
        self.channel_state = ChannelState::Connecting;
        // todo: failure & retry logic
        let stream = TcpStream::connect(self.peer.address).await.unwrap();
        trace!("PeerChannelActor connected to {:?}", self.peer);
        let (reader, writer) = stream.into_split();
        let r_handle = {
            // start the reader task
            let cfg = self.config.clone();
            let cancel = self.subtask_cancel.clone();
            tokio::spawn(async move { PeerChannelActor::reader(self_ref, reader, cfg, cancel).await })
        };
        self.reader_handle = Some(r_handle);
        let (writer_tx, writer_rx) = channel(P2P_COMMS_BUFFER_LENGTH);
        self.writer_tx = Some(writer_tx);
        let w_handle = {
            // start the writer task
            let cfg = self.config.clone();
            let cancel = self.subtask_cancel.clone();
            tokio::spawn(async move { PeerChannelActor::writer(writer_rx, writer, cfg, cancel).await })
        };
        self.writer_handle = Some(w_handle);
        self.channel_state = ChannelState::Handshaking;
        // we send our version straightaway
        let v = Version::default();
        let v_msg = P2PMessage::Version(v);
        self.send_msg(v_msg).await;
        Control::Ok
    }

    async fn handle_sends(&mut self, msg: Self::SendMessage) -> Control {
        use ChannelControlMessage::*;
        match msg {
            PeerMsgReceived(envelope) => {
                self.handle_received(envelope).await;
                Control::Ok
            }
        }
    }

    async fn on_shutdown(&mut self) -> Control {
        self.channel_state = ChannelState::Closing;
        self.subtask_cancel.cancel();
        if self.reader_handle.is_some() {
            let j = self.reader_handle.take().unwrap();
            let _ = j.await;
        }
        if self.writer_handle.is_some() {
            let j = self.writer_handle.take().unwrap();
            let _ = j.await;
        }
        Control::Ok
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
