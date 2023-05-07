use tokio::sync::mpsc::{channel, Sender, Receiver};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use crate::util::ACTOR_CHANNEL_SIZE;

pub struct ListenerConfig {
    pub port: u16,
}

/// A Listener listens for incoming connections from peers.
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
        self.sender.send(ListenerInternalMessage::GetPort { reply: tx }).await;
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
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        loop {
            tokio::select! {
                message = self.inbox.recv() => {
                    match message {
                        Some(ListenerInternalMessage::Stop) => {
                            break;
                        }
                        Some(ListenerInternalMessage::GetPort { reply }) => {
                            reply.send(listener.local_addr().unwrap().port()).unwrap();
                        }
                        None => {
                            println!("ListenerActor: inbox closed, stopping");
                            break;
                        }
                    }
                }
                a = listener.accept() => {
                    match a {
                        Ok((stream, addr)) => {
                            self.outbox.send(ListenerMessage::AcceptConnection { socket: stream, address: addr }).await;
                        }
                        Err(e) => {
                            panic!("Error accepting connection: {}", e);        // todo: handle this better
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::str::FromStr;
    use tokio::net::TcpStream;
    use tokio::time;

    #[tokio::test]
    async fn listener_start_stop() {
        let (tx, rx) = channel(1);
        let (l, j) = Listener::new(tx, ListenerConfig { port: 0 });
        let p = l.get_port().await;
        assert!(p > 0);
        l.sender.send(ListenerInternalMessage::Stop).await;
        j.await.unwrap();
    }

    // test that the listener accepts a connection
    #[tokio::test]
    async fn listener_test_accept() {
        let (tx, mut rx) = channel(1);
        let (l, j) = Listener::new(tx, ListenerConfig { port: 0 });
        let p = l.get_port().await;
        println!("Listener port: {}", p);
        assert!(p > 0);         // did we really get a port back?
        let mut out_stream = TcpStream::connect(SocketAddr::new(IpAddr::from(Ipv4Addr::LOCALHOST), p)).await.unwrap();
        // we expect to receive an AcceptConnection message
        let msg = rx.recv().await.unwrap();
        match msg {
            ListenerMessage::AcceptConnection { socket, address } => {
                println!("Accepted connection from {}", address);
            }
        }
    }
}
