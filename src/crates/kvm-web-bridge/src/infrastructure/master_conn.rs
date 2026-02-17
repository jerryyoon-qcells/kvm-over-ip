//! TCP connection management for the KVM master.
//!
//! Each browser WebSocket session gets its own TCP connection to the master.
//! The master treats the bridge as just another KVM client.
//!
//! # Binary streaming protocol
//!
//! TCP is a *stream* protocol: a single `read()` call may return less than one
//! complete KVM message, or more than one.  This module buffers incoming bytes
//! and uses [`kvm_core::protocol::decode_message`] to extract complete messages
//! from the buffer one at a time.
//!
//! # Portability note
//!
//! This module uses only the `tokio::net::TcpStream` API, which works
//! identically on Windows, Linux, and macOS.  There are no platform-specific
//! syscalls.

use std::net::SocketAddr;

use anyhow::Context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, warn};

use kvm_core::protocol::codec::ProtocolError;
use kvm_core::protocol::messages::KvmMessage;

/// A wrapper around a TCP connection to the KVM master.
///
/// `MasterConnection` owns the read and write halves of the TCP stream and
/// provides higher-level methods for sending and receiving complete KVM
/// messages.
///
/// # Design
///
/// The read and write halves are stored as `Option` so that `into_split` can
/// move them out by value.  Once split, the individual halves are passed to
/// the two forwarding tasks.
pub struct MasterConnection {
    /// Read half of the master TCP stream.
    pub read_half: tokio::net::tcp::OwnedReadHalf,
    /// Write half of the master TCP stream.
    pub write_half: tokio::net::tcp::OwnedWriteHalf,
}

impl MasterConnection {
    /// Opens a new TCP connection to the KVM master at `master_addr`.
    ///
    /// # Errors
    ///
    /// Returns an error if the TCP connection cannot be established (e.g.,
    /// the master is not running, the address is wrong, or a firewall blocks
    /// the connection).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use kvm_web_bridge::infrastructure::master_conn::MasterConnection;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let addr: SocketAddr = "127.0.0.1:24800".parse().unwrap();
    /// let conn = MasterConnection::connect(addr).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(master_addr: SocketAddr) -> anyhow::Result<Self> {
        // `TcpStream::connect` performs the TCP three-way handshake asynchronously.
        // We await it here, which yields control to the Tokio runtime while
        // the network operation is in progress.
        let stream = TcpStream::connect(master_addr)
            .await
            .with_context(|| format!("failed to connect to KVM master at {master_addr}"))?;

        // Split into independent read and write halves so we can pass each to
        // a separate async task without shared ownership.
        let (read_half, write_half) = stream.into_split();

        Ok(Self {
            read_half,
            write_half,
        })
    }
}

// ── Streaming KVM message reader ──────────────────────────────────────────────

/// Reads KVM messages from the master TCP stream and forwards them as JSON.
///
/// This function runs in a loop, accumulating bytes from the TCP stream until
/// complete KVM messages can be decoded.  Each decoded message is translated
/// to JSON and written to the `tx` channel, where the WebSocket write task
/// picks it up and sends it to the browser.
///
/// # Why a buffer is needed
///
/// TCP is a stream protocol.  There is no guarantee that a single `read()` call
/// returns exactly one complete KVM message:
///
/// - It may return fewer bytes than the full message (partial read).
/// - It may return bytes from multiple messages at once (coalesced reads).
///
/// We solve this by accumulating all received bytes in `recv_buf` and calling
/// `decode_message` in a loop until `InsufficientData` tells us we need more.
///
/// # Parameters
///
/// - `read_half`  – Read half of the master TCP stream.
/// - `session_id` – Session identifier string for log messages.
/// - `tx`         – Channel sender: each decoded message is sent here.
///
/// The function returns when the TCP connection is closed (EOF) or an
/// unrecoverable error occurs.
pub async fn read_master_messages(
    mut read_half: tokio::net::tcp::OwnedReadHalf,
    session_id: &str,
    tx: tokio::sync::mpsc::Sender<KvmMessage>,
) {
    // Streaming receive buffer — accumulates bytes across multiple read() calls.
    let mut recv_buf: Vec<u8> = Vec::with_capacity(4096);
    // Temporary read buffer for each individual `read()` syscall.
    let mut read_tmp = vec![0u8; 4096];

    loop {
        // Read more bytes from the master TCP stream.
        // `read()` returns the number of bytes actually read (may be less than
        // `read_tmp.len()` — that is normal and expected).
        let n = match read_half.read(&mut read_tmp).await {
            Ok(0) => {
                // `read()` returned 0 bytes → the master closed the connection (EOF).
                debug!("session {session_id}: master TCP connection closed (EOF)");
                break;
            }
            Ok(n) => n,
            Err(e) => {
                warn!("session {session_id}: read from master failed: {e}");
                break;
            }
        };

        // Append the newly arrived bytes to the accumulation buffer.
        recv_buf.extend_from_slice(&read_tmp[..n]);

        // Attempt to decode one or more complete messages from the buffer.
        // We loop here because a single `read()` may have delivered multiple
        // complete messages at once.
        loop {
            match kvm_core::protocol::decode_message(&recv_buf) {
                Ok((kvm_msg, consumed)) => {
                    debug!(
                        "session {session_id}: decoded KVM message: {:?}",
                        kvm_msg.message_type()
                    );

                    // Remove the consumed bytes from the front of the buffer.
                    // `drain(..consumed)` shifts remaining bytes to the front,
                    // which is O(n) but fine for the small message sizes here.
                    recv_buf.drain(..consumed);

                    // Send the decoded message to the WebSocket write task.
                    // If the receiver has been dropped, the session is over.
                    if tx.send(kvm_msg).await.is_err() {
                        debug!("session {session_id}: message channel closed; exiting reader");
                        return;
                    }
                }
                Err(ProtocolError::InsufficientData { .. }) => {
                    // Normal — we just don't have a full message yet.
                    // Break out of the inner loop and wait for more bytes.
                    break;
                }
                Err(e) => {
                    // A real decode error (unknown message type, corrupt header, etc.).
                    // The connection is likely unsalvageable; close the session.
                    warn!("session {session_id}: KVM decode error from master: {e}");
                    return;
                }
            }
        }
    }
}

/// Writes an encoded KVM message to the master TCP stream.
///
/// This is a thin wrapper around `write_all` that provides a session-ID
/// context for error logging.
///
/// # Errors
///
/// Returns an error if the write fails (e.g., the master closed the connection).
pub async fn write_kvm_message(
    write_half: &mut tokio::net::tcp::OwnedWriteHalf,
    bytes: &[u8],
    session_id: &str,
) -> anyhow::Result<()> {
    // `write_all` ensures ALL bytes are written, even if the OS only accepts
    // a partial write on the first call.  This is important for large messages.
    write_half
        .write_all(bytes)
        .await
        .with_context(|| format!("session {session_id}: write to master failed"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kvm_core::protocol::codec::encode_message;
    use kvm_core::protocol::messages::{KvmMessage, MouseMoveMessage};

    /// Test that the `read_master_messages` loop correctly handles a pre-encoded
    /// message by verifying the decode logic works on a known byte buffer.
    ///
    /// Note: We test the codec-level behavior here since we cannot easily
    /// create a real TCP connection in a unit test.  Integration tests in
    /// `ws_server.rs` test the full network path.
    #[test]
    fn test_kvm_message_encoding_for_master_conn() {
        // Arrange: encode a MouseMove message as the master would produce
        let msg = KvmMessage::MouseMove(MouseMoveMessage {
            x: 100,
            y: 200,
            delta_x: 5,
            delta_y: -3,
        });
        let bytes = encode_message(&msg, 0, 0).unwrap();

        // Act: decode the bytes as the master_conn reader would
        let (decoded, consumed) = kvm_core::protocol::decode_message(&bytes).unwrap();

        // Assert: the message survives the encode→decode round trip
        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_partial_kvm_message_returns_insufficient_data() {
        // Arrange: only provide the first 10 bytes of a 24-byte header
        let msg = KvmMessage::MouseMove(MouseMoveMessage {
            x: 0,
            y: 0,
            delta_x: 0,
            delta_y: 0,
        });
        let bytes = encode_message(&msg, 0, 0).unwrap();
        let partial = &bytes[..10];

        // Act: attempt to decode a partial message
        let result = kvm_core::protocol::decode_message(partial);

        // Assert: the decoder returns InsufficientData, not a panic or garbage
        assert!(matches!(
            result,
            Err(ProtocolError::InsufficientData { .. })
        ));
    }

    #[test]
    fn test_two_messages_in_one_buffer_decode_independently() {
        // Arrange: concatenate two complete messages into one buffer
        // (simulates TCP coalescing multiple sends into one recv)
        let msg1 = KvmMessage::MouseMove(MouseMoveMessage {
            x: 10,
            y: 20,
            delta_x: 1,
            delta_y: 2,
        });
        let msg2 = KvmMessage::MouseMove(MouseMoveMessage {
            x: 30,
            y: 40,
            delta_x: 3,
            delta_y: 4,
        });
        let mut buf = encode_message(&msg1, 0, 0).unwrap();
        buf.extend_from_slice(&encode_message(&msg2, 1, 0).unwrap());

        // Act: decode the first message
        let (decoded1, consumed1) = kvm_core::protocol::decode_message(&buf).unwrap();
        // Advance buffer past the first message
        let remaining = &buf[consumed1..];
        // Decode the second message from the remaining bytes
        let (decoded2, consumed2) = kvm_core::protocol::decode_message(remaining).unwrap();

        // Assert: both messages decode correctly and independently
        assert_eq!(decoded1, msg1);
        assert_eq!(decoded2, msg2);
        assert_eq!(consumed1 + consumed2, buf.len());
    }
}
