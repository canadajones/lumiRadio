use std::path::PathBuf;

use serde::Deserialize;
use tracing::{debug, warn};

use crate::JudeHarleyError;

#[derive(Deserialize, Debug)]
pub struct QueueItem {
    pub album: Option<String>,
    pub artist: String,
    pub title: String,
    pub filename: String,
    pub queue: String,
}

#[async_trait::async_trait]
pub trait LiquidsoapCommunication {
    type Error;
    #[deprecated]
    async fn send(&mut self, command: &str) -> Result<(), Self::Error>;
    async fn send_wait(&mut self, command: &str) -> Result<String, Self::Error>;
    async fn song_requests(&mut self) -> Result<Vec<QueueItem>, Self::Error>;

    async fn request_song(&mut self, song: &str) -> Result<String, Self::Error> {
        self.send_wait(&format!("srq.push {}", song)).await
    }
    async fn priority_request(&mut self, song: &str) -> Result<String, Self::Error> {
        self.send_wait(&format!("prioq.push {}", song)).await
    }
}

pub struct ByersUnixStream {
    stream: tokio::net::UnixStream,
}

impl ByersUnixStream {
    pub async fn new() -> Result<Self, std::io::Error> {
        // wait until /usr/src/app/ls/lumiradio.sock exists
        let stream = loop {
            if PathBuf::from("/usr/src/app/ls/lumiradio.sock").exists() {
                let stream_result = tokio::net::UnixStream::connect(PathBuf::from(
                    "/usr/src/app/ls/lumiradio.sock",
                ))
                .await;
                if let Ok(stream) = stream_result {
                    break stream;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        };

        Ok(Self { stream })
    }

    pub async fn reconnect(&mut self) -> Result<(), std::io::Error> {
        self.stream = Self::new().await?.stream;
        Ok(())
    }

    pub async fn read_until_end(&mut self) -> Result<String, std::io::Error> {
        let mut buf = Vec::new();
        let mut read_buffer = [0; 4096];

        loop {
            self.stream.readable().await?;
            let bytes_read = match self.stream.try_read(&mut read_buffer) {
                Ok(n) => {
                    debug!("Read {} bytes from liquidsoap", n);
                    n
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    debug!("Would block, reading from liquidsoap");
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            };
            buf.extend_from_slice(&read_buffer[..bytes_read]);

            if let Some(end_idx) = buf.windows(3).position(|window| window == b"END") {
                return Ok(String::from_utf8_lossy(&buf[..end_idx]).to_string());
            }

            // clear the read buffer in case we didn't read the whole thing
            read_buffer = [0; 4096];
        }
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        loop {
            self.stream.writable().await?;
            match self.stream.try_write(data) {
                Ok(n) => {
                    debug!("Wrote {} bytes to liquidsoap", n);
                    break;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    debug!("Would block, writing to liquidsoap");
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    pub async fn write_str(&mut self, data: &str) -> Result<(), std::io::Error> {
        debug!("Writing to liquidsoap: {}", data);
        self.write(data.as_bytes()).await
    }

    pub async fn write_line(&mut self, data: &str) -> Result<(), std::io::Error> {
        let data_with_newline = format!("{}\n", data);
        self.write_str(&data_with_newline).await?;

        Ok(())
    }

    pub async fn write_str_and_wait_for_response(
        &mut self,
        data: &str,
    ) -> Result<String, std::io::Error> {
        let data_with_newline = format!("{}\n", data);
        self.write_str(&data_with_newline).await?;

        let result = self.read_until_end().await?;
        Ok(result)
    }
}

#[async_trait::async_trait]
impl LiquidsoapCommunication for ByersUnixStream {
    type Error = JudeHarleyError;

    async fn send_wait(&mut self, command: &str) -> Result<String, Self::Error> {
        let result = self.write_str_and_wait_for_response(command).await;

        if let Err(e) = result {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                warn!("Socket broken, reconnecting");
                self.reconnect().await?;
                return self.send_wait(command).await;
            } else {
                return Err(e.into());
            }
        }

        result.map_err(Into::into)
    }

    #[allow(deprecated)]
    async fn send(&mut self, command: &str) -> Result<(), Self::Error> {
        let result = self.write_line(command).await;

        if let Err(e) = result {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                warn!("Socket broken, reconnecting");
                self.reconnect().await?;
                return self.send(command).await;
            } else {
                return Err(e.into());
            }
        }

        result.map_err(Into::into)
    }

    async fn song_requests(&mut self) -> Result<Vec<QueueItem>, Self::Error> {
        let result = self.send_wait("song_request_queue").await?;
        serde_json::from_str(&result).map_err(Into::into)
    }
}
