use tokio::sync::mpsc::{channel, Sender, Receiver};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use crate::p2p::ACTOR_CHANNEL_SIZE;

// todo: fix and implement

/// Configuration for a P2P Listener.
/// Can be used to specify the port to listen on.
/// Later improvements: specify the address to listen on.
pub struct ListenerConfig {
    pub port: u16,
}

impl ListenerConfig {
    pub fn default() -> Self {
        Self {
            port: 0,        // choose random port
        }
    }
}

/// A Listener listens for incoming connections from peers.
///
/// The Listener listens for incoming connections and sends a ListenerMessage::AcceptConnection
/// message to the supplied outbox when a connection is accepted.
///
/// The Listener is actually a handle to an actor implemented in ListenerActor.
pub struct Listener {
    sender: Sender<ListenerInternalMessage>,
}

impl Listener {
    pub fn new(outbox: Sender<ListenerMessage>, config: ListenerConfig) -> (Self, JoinHandle<()>) {
        let (tx, rx) = channel(ACTOR_CHANNEL_SIZE);
        let mut l = ListenerActor::new(rx, outbox, config);
        let j = tokio::spawn(async move { l.run().await });
        (Listener { sender: tx }, j)
    }

    pub async fn get_port(&self) -> u16 {
        let (tx, rx) = oneshot::channel();
        let _ = self.sender.send(ListenerInternalMessage::GetPort { reply: tx }).await.unwrap();
        rx.await.unwrap()
    }
}

pub enum ListenerMessage {
    AcceptConnection {socket: tokio::net::TcpStream, address: std::net::SocketAddr},
}

enum ListenerInternalMessage {
    Stop,
    GetPort{ reply: oneshot::Sender<u16> }
}

struct ListenerActor {
    inbox: Receiver<ListenerInternalMessage>,
    outbox: Sender<ListenerMessage>,
    config: ListenerConfig,
}

impl ListenerActor {
    fn new(inbox: Receiver<ListenerInternalMessage>, outbox: Sender<ListenerMessage>, config: ListenerConfig) -> Self {
        ListenerActor { inbox, outbox, config }
    }

    async fn run(&mut self) {
        // let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, self.config.port)).await.unwrap();
        // loop {
            // tokio::select! {
                // message = self.inbox.recv() => {
                    // match message {
                    //     Some(ListenerInternalMessage::Stop) => {
                    //         break;
                    //     }
                    //     Some(ListenerInternalMessage::GetPort { reply }) => {
                    //         reply.send(listener.local_addr().unwrap().port()).unwrap();
                    //     }
                    //     None => {
                    //         println!("ListenerActor: inbox closed, stopping");
                    //         break;
                    //     }
                    // }
                // }
                // a = listener.accept() => {
                    // match a {
                        // Ok((stream, addr)) => {
                        //     self.outbox.send(ListenerMessage::AcceptConnection { socket: stream, address: addr }).await;
                        // }
                        // Err(e) => {
                        //     panic!("Error accepting connection: {}", e);        // todo: handle this better
                        // }
                    // }
                // }
            // }
        // }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::net::{IpAddr, Ipv4Addr, SocketAddr};
//     use tokio::net::TcpStream;
//
//     #[tokio::test]
//     async fn listener_start_stop() {
//         let (tx, rx) = channel(1);
//         let (l, j) = Listener::new(tx, ListenerConfig { port: 0 });
//         let p = l.get_port().await;
//         assert!(p > 0);
//         l.sender.send(ListenerInternalMessage::Stop).await;
//         j.await.unwrap();
//     }
//
//     // test that the listener accepts a connection
//     #[tokio::test]
//     async fn listener_test_accept() {
//         let (tx, mut rx) = channel(1);
//         let (l, j) = Listener::new(tx, ListenerConfig { port: 0 });
//         let p = l.get_port().await;
//         println!("Listener port: {}", p);
//         assert!(p > 0);         // did we really get a port back?
//         let mut out_stream = TcpStream::connect(SocketAddr::new(IpAddr::from(Ipv4Addr::LOCALHOST), p)).await.unwrap();
//         // we expect to receive an AcceptConnection message
//         let msg = rx.recv().await.unwrap();
//         match msg {
//             ListenerMessage::AcceptConnection { socket, address } => {
//                 println!("Accepted connection from {}", address);
//             }
//         }
//     }
//
//     // does it actually listen to a specific port?
//     // this might fail because we may be unable to bind to the requested port
//     #[tokio::test]
//     async fn listener_at_port() {
//         let chosen_port = 32631;
//         let (tx, mut rx) = channel(1);
//         let (l, j) = Listener::new(tx, ListenerConfig { port: chosen_port });
//         let p = l.get_port().await;
//         assert_eq!(p, chosen_port);
//         let mut out_stream = TcpStream::connect(SocketAddr::new(IpAddr::from(Ipv4Addr::LOCALHOST), chosen_port)).await.unwrap();
//         // we expect to receive an AcceptConnection message
//         let msg = rx.recv().await.unwrap();
//         match msg {
//             ListenerMessage::AcceptConnection { socket, address } => {
//                 println!("Accepted connection from {}", address);
//             }
//         }
//     }
// }
