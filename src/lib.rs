use core::time::Duration;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener, TcpStream,
    },
    time::sleep,
};
use uuid::Uuid;

const MESSAGE_SIZE: usize = 256;

pub struct Peer {
    incomming: TcpListener,
    outgoing: Option<TcpStream>,
    period: u64,
    message: Message,
    queue: Arc<Mutex<Vec<Message>>>,
    id: Uuid,
}

impl Peer {
    pub async fn new(
        port: u16,
        message: String,
        period: u64,
        outgoing: Option<SocketAddr>,
    ) -> Result<Self> {
        let incomming = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;
        let outgoing = match outgoing {
            Some(addr) => Some(TcpStream::connect(addr).await?),
            None => Default::default(),
        };
        let id = Uuid::new_v4();
        let message = Message {
            message,
            peer_id: id,
        };

        Ok(Self {
            incomming,
            period,
            message,
            outgoing,
            queue: Default::default(),
            id,
        })
    }

    pub async fn listen(&mut self) -> Result<()> {
        let period = self.period;
        let message = self.message.clone();
        let id = self.id;

        let queue = Arc::clone(&self.queue);
        tokio::spawn(async move {
            Self::queue_message(message.clone(), period, queue).await;
        });
        if let Some(socket) = self.outgoing.take() {
            let (reader, writer) = socket.into_split();
            let address = self.incomming.local_addr()?;

            let queue = Arc::clone(&self.queue);
            tokio::spawn(async move {
                Self::read_messages(reader, address, Some(queue), id).await;
            });

            let queue = Arc::clone(&self.queue);
            tokio::spawn(async move {
                Self::process_queue(writer, queue).await;
            });
        }

        loop {
            if let Ok((socket, address)) = self.incomming.accept().await {
                log::info!(
                    "New connection from {} to {}",
                    address,
                    self.incomming.local_addr()?
                );

                let (reader, writer) = socket.into_split();

                let queue = Arc::clone(&self.queue);

                tokio::spawn(async move {
                    Self::read_messages(reader, address, Some(queue), id).await;
                });

                let queue = Arc::clone(&self.queue);
                tokio::spawn(async move {
                    Self::process_queue(writer, queue).await;
                });
            }
        }

        // Ok(())
    }

    async fn read_messages(
        mut socket: OwnedReadHalf,
        address: SocketAddr,
        queue: Option<Arc<Mutex<Vec<Message>>>>,
        peer_id: Uuid,
    ) {
        let mut stream = BufReader::new(socket);
        loop {
            // let mut line = String::new();
            // let mut buf = [0; MESSAGE_SIZE];
            let mut buf = Vec::new();

            loop {
                match stream.read_until(b'}', &mut buf).await {
                    Ok(s) => {
                        if s > 0 {
                            let message = String::from_utf8_lossy(&buf);
                            let message: Message = match serde_json::from_str(&message) {
                                Ok(m) => m,
                                Err(e) => {
                                    log::error!("Error deserializing message {}: {}", message, e);
                                    break;
                                }
                            };

                            if peer_id == message.peer_id {
                                continue;
                            }

                            log::warn!("got message from {}: {}", address, message.message);

                            if let Some(ref queue) = queue {
                                queue.lock().unwrap().push(message);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error reading from {}: {}", address, e);
                        break;
                    }
                }
            }
        }
    }

    async fn process_queue(mut socket: OwnedWriteHalf, queue: Arc<Mutex<Vec<Message>>>) {
        loop {
            let message = queue.lock().unwrap().pop();
            if let Some(message) = message {
                // let mut buf = [0; MESSAGE_SIZE];
                let message = match serde_json::to_string(&message) {
                    Ok(m) => m,
                    Err(e) => {
                        log::error!("Error serializing message: {}", e);
                        break;
                    }
                };

                // if message.len() > MESSAGE_SIZE {
                    // log::error!("Message is too large");
                    // break;
                // }
// 
                // buf[..message.len()].copy_from_slice(message.as_bytes());

                if let Err(e) = socket.write_all(&message.as_bytes()).await {
                    log::error!("Error writing message: {}", e);
                    break;
                }
            }
        }
    }

    async fn queue_message(message: Message, period: u64, queue: Arc<Mutex<Vec<Message>>>) {
        loop {
            sleep(Duration::from_secs(period)).await;
            queue.lock().unwrap().push(message.clone());
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Message {
    message: String,
    peer_id: Uuid,
}
