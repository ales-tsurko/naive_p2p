use core::time::Duration;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener, TcpStream,
    },
    time::sleep,
};
use uuid::Uuid;

pub struct Peer {
    incomming: TcpListener,
    outgoing: Option<TcpStream>,
    message: Message,
    queue: Arc<Mutex<Vec<Message>>>,
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
        let message = Message {
            message,
            period,
            id: Uuid::new_v4(),
        };

        Ok(Self {
            incomming,
            message,
            outgoing,
            queue: Default::default(),
        })
    }

    pub async fn listen(&mut self) -> Result<()> {
        let message = self.message.clone();
        let queue = Arc::clone(&self.queue);

        tokio::spawn(async move {
            Self::queue_message(message, queue).await;
        });

        self.be_client().await?;

        self.be_server().await?;

        Ok(())
    }

    async fn queue_message(message: Message, queue: Arc<Mutex<Vec<Message>>>) {
        loop {
            sleep(Duration::from_secs(message.period)).await;
            queue.lock().unwrap().push(message.clone());
        }
    }

    async fn be_client(&mut self) -> Result<()> {
        if let Some(socket) = self.outgoing.take() {
            let (reader, writer) = socket.into_split();
            let address = self.incomming.local_addr()?;

            let queue = Arc::clone(&self.queue);
            let id = self.message.id;
            tokio::spawn(async move {
                if let Err(e) = Self::read_messages(reader, address, queue, id).await {
                    log::error!("{}", e);
                }
            });

            let queue = Arc::clone(&self.queue);
            tokio::spawn(async move {
                if let Err(e) = Self::process_queue(writer, queue).await {
                    log::error!("{}", e);
                }
            });
        }

        Ok(())
    }

    async fn be_server(&mut self) -> Result<()> {
        let id = self.message.id;

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
                    if let Err(e) = Self::read_messages(reader, address, queue, id).await {
                        log::error!("{}", e);
                    }
                });

                let queue = Arc::clone(&self.queue);
                tokio::spawn(async move {
                    if let Err(e) = Self::process_queue(writer, queue).await {
                        log::error!("{}", e);
                    }
                });
            }
        }
    }

    async fn read_messages(
        socket: OwnedReadHalf,
        address: SocketAddr,
        queue: Arc<Mutex<Vec<Message>>>,
        id: Uuid,
    ) -> Result<(), Error> {
        let mut stream = BufReader::new(socket);
        let mut buf = Vec::new();

        loop {
            if stream
                .read_until(b'}', &mut buf)
                .await
                .map_err(|e| Error::ReadingMessage(address.to_string(), e.to_string()))?
                == 0
            {
                continue;
            }

            let message = String::from_utf8_lossy(&buf);
            let message: Message = serde_json::from_str(&message)
                .map_err(|e| Error::DeserializingMessage(message.to_string(), e.to_string()))?;

            buf.clear();

            if id == message.id {
                continue;
            }

            log::warn!("got message from {}: {}", address, message.message);

            queue.lock().unwrap().push(message);
        }
    }

    async fn process_queue(
        mut socket: OwnedWriteHalf,
        queue: Arc<Mutex<Vec<Message>>>,
    ) -> Result<(), Error> {
        loop {
            let message = queue.lock().unwrap().pop();
            if let Some(message) = message {
                let message = serde_json::to_string(&message)
                    .map_err(|e| Error::SerializingMessage(e.to_string()))?;

                socket
                    .write_all(&message.as_bytes())
                    .await
                    .map_err(|e| Error::WritingMessage(e.to_string()))?;
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Message {
    message: String,
    id: Uuid,
    period: u64,
}

#[derive(Debug, Error)]
enum Error {
    #[error("Error reading message from {0}: {1}")]
    ReadingMessage(String, String),
    #[error("Error writing message: {0}")]
    WritingMessage(String),
    #[error("Error serializing message: {0}")]
    SerializingMessage(String),
    #[error("Error deserializing message {0}: {1}")]
    DeserializingMessage(String, String),
}
