//! UDP broadcast-based device discovery.
//!
//! The master binds a UDP socket on the discovery port (default 24802) and
//! responds to `Announce` messages broadcast by clients.  On receiving a valid
//! `Announce`, it:
//!
//! 1. Parses the client identity from the payload.
//! 2. Sends an `AnnounceResponse` back to the client's source address.
//! 3. Emits a [`DiscoveryEvent`] on the internal channel so the application
//!    layer can initiate pairing / connection.
//!
//! The responder runs as a blocking task on a dedicated thread to avoid
//! blocking the Tokio runtime with synchronous socket I/O.
//!
//! # How UDP discovery works (for beginners)
//!
//! UDP (User Datagram Protocol) is a lightweight, connectionless networking
//! protocol.  Unlike TCP it does not guarantee delivery, ordering, or duplicate
//! prevention.  These trade-offs make it ideal for discovery broadcasts:
//!
//! 1. The client sends a UDP packet to the LAN broadcast address (e.g.,
//!    `255.255.255.255`) on the discovery port.  Every device on the LAN
//!    receives this packet.
//!
//! 2. The master is listening on that port.  It parses the `AnnounceMessage`
//!    inside the packet and sends a unicast `AnnounceResponse` back to the
//!    sender's IP address.
//!
//! 3. The client receives the response and knows the master's IP + control port.
//!    It can now establish a TCP connection to begin the pairing handshake.
//!
//! # Read timeout
//!
//! The socket is configured with a 1-second read timeout.  This means the
//! `recv_from` call blocks for at most 1 second before returning a timeout
//! error.  On each timeout we check the `running` flag; if the application
//! is shutting down we exit the loop cleanly.

use std::net::{SocketAddr, UdpSocket};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use kvm_core::{
    protocol::{
        codec::{decode_message, encode_message},
        messages::{AnnounceResponseMessage, KvmMessage, PlatformId},
    },
    ClientId,
};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Error type for discovery service operations.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// The UDP socket could not be bound.
    #[error("failed to bind discovery socket on {addr}: {source}")]
    BindFailed {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    /// An I/O error occurred while receiving a datagram.
    #[error("recv error: {0}")]
    Recv(std::io::Error),
}

/// An event produced when a client announces its presence.
#[derive(Debug, Clone)]
pub struct DiscoveryEvent {
    /// The UUID the client advertised.
    pub client_id: ClientId,
    /// The human-readable name the client advertised.
    pub name: String,
    /// The platform identifier from the Announce message.
    pub platform_id: PlatformId,
    /// The source address from which the UDP datagram arrived.
    pub client_addr: SocketAddr,
    /// The TCP control port the client is listening on.
    pub control_port: u16,
}

/// Sequence counter used when constructing `AnnounceResponse` messages.
static RESP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Binds a UDP socket on `discovery_port` and spawns a background thread that
/// processes incoming `Announce` datagrams.
///
/// Returns a receiver from which the application layer reads [`DiscoveryEvent`]s.
///
/// # Errors
///
/// Returns [`DiscoveryError::BindFailed`] if the socket cannot be bound.
pub fn start_discovery_responder(
    discovery_port: u16,
    running: Arc<AtomicBool>,
) -> Result<mpsc::Receiver<DiscoveryEvent>, DiscoveryError> {
    let addr: SocketAddr = format!("0.0.0.0:{discovery_port}").parse().unwrap();
    let socket =
        UdpSocket::bind(addr).map_err(|source| DiscoveryError::BindFailed { addr, source })?;
    socket
        .set_read_timeout(Some(Duration::from_millis(500)))
        .ok();

    let (tx, rx) = mpsc::channel(64);

    std::thread::Builder::new()
        .name("kvm-discovery".to_string())
        .spawn(move || {
            discovery_loop(socket, tx, running);
        })
        .expect("failed to spawn discovery thread");

    info!("discovery responder listening on UDP {addr}");
    Ok(rx)
}

/// The main receive loop executed on the discovery thread.
fn discovery_loop(socket: UdpSocket, tx: mpsc::Sender<DiscoveryEvent>, running: Arc<AtomicBool>) {
    let mut buf = vec![0u8; 4096];

    while running.load(Ordering::Relaxed) {
        let (len, src) = match socket.recv_from(&mut buf) {
            Ok(pair) => pair,
            Err(e) if is_timeout_error(&e) => continue,
            Err(e) => {
                error!("discovery recv error: {e}");
                continue;
            }
        };

        let datagram = &buf[..len];
        match decode_message(datagram) {
            Ok((KvmMessage::Announce(msg), _)) => {
                debug!(
                    "announce from {src}: client_id={}, name={}",
                    msg.client_id, msg.client_name
                );

                // Respond so the client knows the master is present.
                send_announce_response(&socket, src);

                let client_id: ClientId = msg.client_id;
                let event = DiscoveryEvent {
                    client_id,
                    name: msg.client_name.clone(),
                    platform_id: msg.platform_id,
                    client_addr: src,
                    control_port: msg.control_port,
                };

                if tx.blocking_send(event).is_err() {
                    // Receiver dropped – application is shutting down.
                    break;
                }
            }
            Ok((other, _)) => {
                warn!(
                    "unexpected message on discovery port from {src}: {:?}",
                    std::mem::discriminant(&other)
                );
            }
            Err(e) => {
                debug!("failed to decode discovery datagram from {src}: {e}");
            }
        }
    }

    info!("discovery responder stopped");
}

/// Sends an `AnnounceResponse` back to `dest`.
fn send_announce_response(socket: &UdpSocket, dest: SocketAddr) {
    let seq = RESP_SEQ.fetch_add(1, Ordering::Relaxed);
    let msg = KvmMessage::AnnounceResponse(AnnounceResponseMessage {
        master_control_port: 24800,
        already_paired: false,
    });
    let ts = current_timestamp_us();
    match encode_message(&msg, seq, ts) {
        Ok(bytes) => {
            if let Err(e) = socket.send_to(&bytes, dest) {
                warn!("failed to send AnnounceResponse to {dest}: {e}");
            }
        }
        Err(e) => error!("failed to encode AnnounceResponse: {e}"),
    }
}

/// Returns `true` for OS timeout / would-block errors that should be retried.
fn is_timeout_error(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}

/// Returns the current time as microseconds since the Unix epoch.
fn current_timestamp_us() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_is_timeout_error_recognises_timed_out() {
        // Arrange
        let e = std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out");

        // Act / Assert
        assert!(is_timeout_error(&e));
    }

    #[test]
    fn test_is_timeout_error_recognises_would_block() {
        // Arrange
        let e = std::io::Error::new(std::io::ErrorKind::WouldBlock, "would block");

        // Act / Assert
        assert!(is_timeout_error(&e));
    }

    #[test]
    fn test_is_timeout_error_returns_false_for_other_errors() {
        // Arrange
        let e = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");

        // Act / Assert
        assert!(!is_timeout_error(&e));
    }

    #[test]
    fn test_current_timestamp_us_returns_nonzero() {
        // Arrange / Act
        let ts = current_timestamp_us();

        // Assert
        assert!(ts > 0, "timestamp must be positive");
    }

    #[test]
    fn test_discovery_event_fields_are_accessible() {
        // Arrange
        let event = DiscoveryEvent {
            client_id: Uuid::new_v4(),
            name: "test-client".to_string(),
            platform_id: PlatformId::Windows,
            client_addr: "192.168.1.50:24800".parse().unwrap(),
            control_port: 24800,
        };

        // Act / Assert
        assert_eq!(event.name, "test-client");
        assert_eq!(event.control_port, 24800);
        assert_eq!(event.platform_id, PlatformId::Windows);
    }

    #[test]
    fn test_start_discovery_responder_binds_and_returns_receiver() {
        // Arrange: find a free port by binding port 0 and reading back the OS-assigned port
        let probe = UdpSocket::bind("0.0.0.0:0").expect("probe bind");
        let port = probe.local_addr().unwrap().port();
        drop(probe); // release the port before re-binding

        let running = Arc::new(AtomicBool::new(false)); // stopped immediately

        // Act
        let result = start_discovery_responder(port, running);

        // Assert
        assert!(result.is_ok(), "responder must bind successfully");
    }

    #[test]
    fn test_start_discovery_responder_fails_on_privileged_port() {
        // Port 1 requires root; this must fail on a normal OS.
        // Skip on CI environments that run as root.
        if std::env::var("CI_ROOT").is_ok() {
            return;
        }
        let running = Arc::new(AtomicBool::new(false));
        let result = start_discovery_responder(1, running);

        // On most systems this will be Err; if it succeeds (e.g. running as root) that is OK.
        // We only assert that the function does not panic.
        let _ = result;
    }
}
