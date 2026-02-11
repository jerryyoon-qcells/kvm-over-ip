//! Network infrastructure for the client application.
//!
//! Handles the TCP control channel connection to the master and dispatches
//! inbound [`KvmMessage`]s to the application layer.
//!
//! Architecture:
//! - `ClientConnection` owns a TCP stream (control channel).
//! - Inbound messages are decoded and forwarded on an `mpsc` channel.
//! - Outbound messages (e.g. `ScreenInfo`, `Ping`) are sent through the
//!   connection.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use kvm_core::{
    decode_message, encode_message,
    protocol::messages::{
        capabilities, HelloMessage, KvmMessage, PlatformId, ScreenInfoMessage,
    },
};
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, Mutex},
    time,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Errors that can occur in the client network layer.
#[derive(Debug, Error)]
pub enum ClientNetworkError {
    /// TCP connection to the master failed.
    #[error("failed to connect to master at {addr}: {source}")]
    ConnectFailed {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    /// An I/O error occurred on the established connection.
    #[error("connection I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// A message could not be encoded or decoded.
    #[error("protocol error: {0}")]
    Protocol(String),
    /// The connection was closed by the remote side.
    #[error("connection closed by master")]
    Closed,
}

/// Configuration for the client's network connection.
#[derive(Debug, Clone)]
pub struct ClientConnectionConfig {
    /// Address of the master's TCP control port.
    pub master_addr: SocketAddr,
    /// This client's UUID.
    pub client_id: Uuid,
    /// Human-readable name advertised to the master.
    pub client_name: String,
    /// Reconnect interval when the connection drops.
    pub reconnect_interval: Duration,
}

impl Default for ClientConnectionConfig {
    fn default() -> Self {
        Self {
            master_addr: "127.0.0.1:24800".parse().unwrap(),
            client_id: Uuid::nil(),
            client_name: "kvm-client".to_string(),
            reconnect_interval: Duration::from_secs(5),
        }
    }
}

/// Events emitted by the network layer to the application layer.
#[derive(Debug)]
pub enum NetworkEvent {
    /// A message was received from the master.
    MessageReceived(KvmMessage),
    /// The TCP connection was established.
    Connected { master_addr: SocketAddr },
    /// The TCP connection was lost.
    Disconnected,
}

/// Sequence counter for outbound messages.
static OUTBOUND_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_seq() -> u64 {
    OUTBOUND_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Returns the [`PlatformId`] for the current compilation target.
fn native_platform_id() -> PlatformId {
    #[cfg(target_os = "windows")]
    return PlatformId::Windows;
    #[cfg(target_os = "linux")]
    return PlatformId::Linux;
    #[cfg(target_os = "macos")]
    return PlatformId::MacOs;
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return PlatformId::Web;
}

fn current_timestamp_us() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}

/// Manages the TCP control-channel connection from the client to the master.
pub struct ClientConnection {
    config: ClientConnectionConfig,
    write_half: Arc<Mutex<Option<tokio::net::tcp::OwnedWriteHalf>>>,
}

impl ClientConnection {
    /// Creates a new (not yet connected) `ClientConnection`.
    pub fn new(config: ClientConnectionConfig) -> Self {
        Self {
            config,
            write_half: Arc::new(Mutex::new(None)),
        }
    }

    /// Connects to the master and begins reading messages.
    ///
    /// Returns a channel receiver that delivers [`NetworkEvent`]s to the caller.
    /// Runs a continuous reconnect loop until `running` is set to false.
    pub async fn start(
        self: Arc<Self>,
        running: Arc<std::sync::atomic::AtomicBool>,
    ) -> mpsc::Receiver<NetworkEvent> {
        let (tx, rx) = mpsc::channel(128);
        let this = Arc::clone(&self);

        tokio::spawn(async move {
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                match TcpStream::connect(this.config.master_addr).await {
                    Ok(stream) => {
                        info!("connected to master at {}", this.config.master_addr);
                        let addr = this.config.master_addr;
                        let _ = tx.send(NetworkEvent::Connected { master_addr: addr }).await;

                        let (read_half, write_half_owned) = stream.into_split();
                        {
                            let mut guard = this.write_half.lock().await;
                            *guard = Some(write_half_owned);
                        }

                        // Send Hello handshake
                        this.send_hello().await;

                        // Drive the read loop
                        this.read_loop(read_half, &tx).await;

                        {
                            let mut guard = this.write_half.lock().await;
                            *guard = None;
                        }
                        let _ = tx.send(NetworkEvent::Disconnected).await;
                        info!("disconnected from master; reconnecting in {:?}", this.config.reconnect_interval);
                    }
                    Err(e) => {
                        warn!("could not connect to master at {}: {e}", this.config.master_addr);
                    }
                }

                if running.load(std::sync::atomic::Ordering::Relaxed) {
                    time::sleep(this.config.reconnect_interval).await;
                }
            }
        });

        rx
    }

    /// Sends the `Hello` handshake message.
    async fn send_hello(&self) {
        let msg = KvmMessage::Hello(HelloMessage {
            client_id: self.config.client_id,
            client_name: self.config.client_name.clone(),
            protocol_version: kvm_core::protocol::messages::PROTOCOL_VERSION,
            platform_id: native_platform_id(),
            capabilities: capabilities::KEYBOARD_EMULATION
                | capabilities::MOUSE_EMULATION
                | capabilities::MULTI_MONITOR,
        });
        self.send_message(&msg).await;
    }

    /// Reads messages from the TCP stream and forwards them on `tx`.
    async fn read_loop(
        &self,
        mut reader: tokio::net::tcp::OwnedReadHalf,
        tx: &mpsc::Sender<NetworkEvent>,
    ) {
        // We need a length-prefixed framing layer.  The protocol header contains
        // payload_len at bytes 4..8, so we first read the 24-byte header, then
        // payload_len more bytes.
        const HEADER_SIZE: usize = kvm_core::protocol::messages::HEADER_SIZE;

        loop {
            let mut header_buf = vec![0u8; HEADER_SIZE];
            if let Err(e) = reader.read_exact(&mut header_buf).await {
                if e.kind() != std::io::ErrorKind::UnexpectedEof {
                    error!("read error on control channel: {e}");
                }
                break;
            }

            // Payload length is at bytes 4..8 (big-endian u32)
            let payload_len =
                u32::from_be_bytes(header_buf[4..8].try_into().unwrap()) as usize;

            let mut full_msg = header_buf;
            full_msg.extend(vec![0u8; payload_len]);
            if payload_len > 0 {
                if let Err(e) = reader
                    .read_exact(&mut full_msg[HEADER_SIZE..])
                    .await
                {
                    error!("read payload error: {e}");
                    break;
                }
            }

            match decode_message(&full_msg) {
                Ok((msg, _)) => {
                    debug!("received {:?}", std::mem::discriminant(&msg));

                    // Auto-respond to Ping with Pong carrying the same sequence number.
                    if let KvmMessage::Ping(seq) = msg {
                        let pong = KvmMessage::Pong(seq);
                        self.send_message(&pong).await;
                        // Forward the Ping to the application layer so it can track latency.
                        if tx.send(NetworkEvent::MessageReceived(KvmMessage::Ping(seq))).await.is_err() {
                            break;
                        }
                    } else if tx.send(NetworkEvent::MessageReceived(msg)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    warn!("failed to decode inbound message: {e}");
                }
            }
        }
    }

    /// Encodes and sends a message on the control channel.
    pub async fn send_message(&self, msg: &KvmMessage) {
        let ts = current_timestamp_us();
        match encode_message(msg, next_seq(), ts) {
            Ok(bytes) => {
                let mut guard = self.write_half.lock().await;
                if let Some(ref mut w) = *guard {
                    if let Err(e) = w.write_all(&bytes).await {
                        error!("failed to send message: {e}");
                    }
                }
            }
            Err(e) => error!("failed to encode message: {e}"),
        }
    }

    /// Sends a `ScreenInfo` report to the master.
    ///
    /// The `ScreenInfoMessage` is built by the `screen_info` infrastructure module.
    pub async fn send_screen_info(&self, screen_info: ScreenInfoMessage) {
        self.send_message(&KvmMessage::ScreenInfo(screen_info)).await;
    }

    /// Sends a `Ping` to measure round-trip latency.
    ///
    /// The sequence number in the `Ping` payload is used to match
    /// the corresponding `Pong` response.
    pub async fn send_ping(&self) {
        let seq = next_seq();
        self.send_message(&KvmMessage::Ping(seq)).await;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_connection_config_default_has_expected_port() {
        // Arrange / Act
        let cfg = ClientConnectionConfig::default();

        // Assert
        assert_eq!(cfg.master_addr.port(), 24800);
    }

    #[test]
    fn test_client_connection_config_default_reconnect_interval_is_five_seconds() {
        let cfg = ClientConnectionConfig::default();
        assert_eq!(cfg.reconnect_interval, Duration::from_secs(5));
    }

    #[test]
    fn test_next_seq_increments_monotonically() {
        // Arrange
        let a = next_seq();
        let b = next_seq();

        // Assert
        assert!(b > a, "sequence must be monotonically increasing");
    }

    #[test]
    fn test_current_timestamp_us_is_positive() {
        // Arrange / Act
        let ts = current_timestamp_us();

        // Assert
        assert!(ts > 0);
    }

    #[test]
    fn test_network_event_message_received_holds_message() {
        // Arrange
        let msg = KvmMessage::Pong(42);
        let event = NetworkEvent::MessageReceived(msg);

        // Assert – pattern-match to confirm the variant carries the value
        if let NetworkEvent::MessageReceived(KvmMessage::Pong(seq)) = event {
            assert_eq!(seq, 42);
        } else {
            panic!("unexpected event variant");
        }
    }

    #[test]
    fn test_new_client_connection_write_half_is_none() {
        // Arrange
        let cfg = ClientConnectionConfig::default();

        // Act
        let conn = ClientConnection::new(cfg);

        // Assert – write_half must start as None (not connected)
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let guard = conn.write_half.lock().await;
            assert!(guard.is_none(), "write half must be None before connecting");
        });
    }

    #[tokio::test]
    async fn test_start_returns_receiver_immediately() {
        // Arrange
        let cfg = ClientConnectionConfig {
            // Use an address that will refuse connection immediately
            master_addr: "127.0.0.1:1".parse().unwrap(),
            reconnect_interval: Duration::from_secs(60),
            ..Default::default()
        };
        let running = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let conn = Arc::new(ClientConnection::new(cfg));

        // Act – start returns a receiver synchronously even if the TCP connect fails
        let rx = conn.start(Arc::clone(&running)).await;

        // Assert – the receiver was created (we can check it exists)
        drop(rx);
    }
}
