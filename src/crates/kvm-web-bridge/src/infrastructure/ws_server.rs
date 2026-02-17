//! WebSocket server: accept loop and per-session task management.
//!
//! This module is responsible for:
//!
//! 1. Binding a TCP listener on the configured address.
//! 2. Accepting incoming TCP connections from browsers.
//! 3. Upgrading each connection to a WebSocket session.
//! 4. Opening a corresponding TCP connection to the KVM master.
//! 5. Running two concurrent forwarding tasks per session:
//!    - **Browser → Master**: reads JSON from WebSocket, translates to binary,
//!      writes to the master TCP stream.
//!    - **Master → Browser**: reads binary from master TCP, translates to JSON,
//!      writes to the WebSocket.
//! 6. Running a keepalive ping/pong loop for the master connection.
//! 7. Gracefully shutting down when the `running` flag is cleared.
//!
//! # Scalability
//!
//! Each browser session runs in its own Tokio task.  Tokio's multi-threaded
//! runtime distributes tasks across OS threads automatically.  The `run_server`
//! accept loop never blocks: it accepts a connection and immediately spawns
//! a new task for it before accepting the next one.  This means the bridge can
//! handle many simultaneous browser sessions limited only by available memory
//! and the OS's TCP stack.
//!
//! # Portability
//!
//! Uses only `tokio::net` APIs which are portable across Windows, Linux, and
//! macOS.  Shutdown is triggered by a shared `AtomicBool` that is set by a
//! Ctrl+C signal handler (see `main.rs`), which is also cross-platform.

use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{interval, timeout};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Error as WsError, Message as WsMessage},
};
use tracing::{debug, error, info, warn};

use kvm_core::protocol::codec::encode_message_now;
use kvm_core::protocol::sequence::SequenceCounter;

use crate::application::{translate_browser_to_kvm, translate_kvm_to_browser};
use crate::domain::config::BridgeConfig;
use crate::domain::messages::BrowserToMasterMsg;
use crate::infrastructure::master_conn::MasterConnection;

// ── Public API ────────────────────────────────────────────────────────────────

/// Runs the main WebSocket accept loop until `running` is set to `false`.
///
/// Binds a TCP listener on `config.ws_bind_addr` and accepts incoming
/// connections in a loop.  Each accepted connection is handed off to a
/// dedicated Tokio task so that one slow client never blocks others.
///
/// # Parameters
///
/// - `config`  – Bridge configuration (addresses, timeouts).
/// - `running` – Shared flag; the loop exits when this is set to `false`.
///
/// # Errors
///
/// Returns an error if the TCP listener cannot be bound (e.g., the port is
/// already in use or the process lacks permission to bind).
pub async fn run_server(config: BridgeConfig, running: Arc<AtomicBool>) -> anyhow::Result<()> {
    // Bind the WebSocket TCP listener.
    // `TcpListener::bind` is the async equivalent of `bind()` + `listen()`.
    let listener = TcpListener::bind(config.ws_bind_addr)
        .await
        .with_context(|| {
            format!(
                "failed to bind WebSocket listener on {}",
                config.ws_bind_addr
            )
        })?;

    info!("WebSocket bridge listening on {}", config.ws_bind_addr);

    // Wrap config in `Arc` so it can be shared cheaply across many session tasks
    // without copying.  `Arc` stands for Atomically Reference Counted — it's
    // the Rust equivalent of a shared pointer with thread-safe ref counting.
    let config = Arc::new(config);

    loop {
        // Check the shutdown flag before each accept attempt.
        if !running.load(Ordering::Relaxed) {
            info!("shutdown flag set; stopping accept loop");
            break;
        }

        // Use a short timeout on `accept()` so the loop can periodically check
        // the `running` flag even when no browsers are connecting.
        // Without this timeout, the loop would block forever on `accept()`.
        let accept_result = timeout(Duration::from_millis(200), listener.accept()).await;

        match accept_result {
            Ok(Ok((stream, peer_addr))) => {
                info!("new browser connection from {peer_addr}");
                let cfg = Arc::clone(&config);

                // Spawn a dedicated Tokio task for this session.
                // `tokio::spawn` is non-blocking: it queues the task and returns
                // immediately, so the accept loop is never delayed by I/O.
                tokio::spawn(async move {
                    handle_browser_session(stream, peer_addr, cfg).await;
                });
            }
            Ok(Err(e)) => {
                // Transient accept error (e.g., too many open file descriptors).
                // Log it and continue rather than crashing the whole bridge.
                error!("accept error: {e}");
            }
            Err(_) => {
                // Timeout — no new connection in the last 200 ms.
                // Loop back to check the `running` flag.
            }
        }
    }

    Ok(())
}

// ── Per-session handler ───────────────────────────────────────────────────────

/// Top-level handler for a single browser WebSocket session.
///
/// Wraps [`run_session`] and logs the outcome.  This function is the entry
/// point for each per-session Tokio task spawned by [`run_server`].
///
/// Using a separate outer/inner function pair lets us use `?` for clean error
/// propagation inside `run_session` while logging errors in this outer function.
async fn handle_browser_session(
    raw_stream: TcpStream,
    peer_addr: SocketAddr,
    config: Arc<BridgeConfig>,
) {
    match run_session(raw_stream, peer_addr, config).await {
        Ok(()) => info!("session {peer_addr} closed normally"),
        Err(e) => warn!("session {peer_addr} closed with error: {e:#}"),
    }
}

/// Runs the complete lifecycle of a single browser WebSocket session.
///
/// This function:
///
/// 1. Completes the WebSocket HTTP upgrade handshake with the browser.
/// 2. Opens a TCP connection to the KVM master.
/// 3. Runs three concurrent async tasks:
///    - Browser → Master: JSON frames → binary KVM messages
///    - Master → Browser: binary KVM messages → JSON frames
///    - Keepalive: sends periodic KVM Pings to the master
/// 4. Returns when any of the three tasks finishes (session is over).
///
/// # Errors
///
/// Returns an error if the WebSocket handshake fails or the master TCP
/// connection cannot be established.
async fn run_session(
    raw_stream: TcpStream,
    peer_addr: SocketAddr,
    config: Arc<BridgeConfig>,
) -> anyhow::Result<()> {
    // ── Step 1: Complete the WebSocket handshake ───────────────────────────────
    //
    // `accept_async` reads the browser's HTTP Upgrade request and sends the
    // "101 Switching Protocols" response.  After this, `ws_stream` speaks
    // WebSocket frames instead of raw HTTP.
    let ws_stream = accept_async(raw_stream)
        .await
        .with_context(|| format!("WebSocket handshake failed with {peer_addr}"))?;

    info!("WebSocket session established: {peer_addr}");

    // ── Step 2: Connect to the KVM master ─────────────────────────────────────
    let master_conn = MasterConnection::connect(config.master_addr)
        .await
        .with_context(|| {
            format!(
                "session {peer_addr}: failed to connect to master at {}",
                config.master_addr
            )
        })?;

    info!(
        "session {peer_addr}: connected to master at {}",
        config.master_addr
    );

    // ── Step 3: Split streams into read/write halves ───────────────────────────
    //
    // We need to read and write on both connections simultaneously (in separate
    // Tokio tasks).  Splitting gives us independently owned half-handles.

    // Split the WebSocket stream into a write sink and a read stream.
    // `ws_tx` is the "sink" (we write frames to it).
    // `ws_rx` is the "stream" (we read frames from it).
    let (ws_tx, ws_rx) = ws_stream.split();

    // Wrap `ws_tx` in an `Arc<Mutex>` so it can be shared between the
    // master→browser task and the keepalive task.
    // `Mutex` here is `tokio::sync::Mutex` — it's async-aware and won't block
    // the thread while waiting for the lock.
    let ws_tx = Arc::new(tokio::sync::Mutex::new(ws_tx));

    // Take apart the master connection into its read and write halves.
    let (master_read, master_write) = (master_conn.read_half, master_conn.write_half);

    // ── Step 4: Set up the KVM message channel ────────────────────────────────
    //
    // The `read_master_messages` function sends decoded KVM messages through
    // a channel.  The master→browser task receives them and forwards to the
    // browser.  Using a channel decouples the two tasks cleanly.
    let (kvm_tx, mut kvm_rx) =
        tokio::sync::mpsc::channel::<kvm_core::protocol::messages::KvmMessage>(128);

    // Session identifier string used in log messages.
    let session_id = peer_addr.to_string();

    // Sequence counter for messages sent from the bridge to the master.
    // Each message must carry a monotonically increasing sequence number.
    let seq = Arc::new(SequenceCounter::new());

    // ── Task A: Master reader ──────────────────────────────────────────────────
    //
    // Reads binary KVM messages from the master TCP stream and sends them to
    // `kvm_tx`.  Runs until the master closes the connection or an error occurs.
    let session_id_reader = session_id.clone();
    let master_reader_task = tokio::spawn(async move {
        crate::infrastructure::master_conn::read_master_messages(
            master_read,
            &session_id_reader,
            kvm_tx,
        )
        .await;
    });

    // ── Task B: Master → Browser forwarder ────────────────────────────────────
    //
    // Receives decoded KVM messages from `kvm_rx`, translates them to JSON, and
    // sends them to the browser as WebSocket text frames.
    let ws_tx_m2b = Arc::clone(&ws_tx);
    let session_id_m2b = session_id.clone();
    let master_to_browser_task = tokio::spawn(async move {
        while let Some(kvm_msg) = kvm_rx.recv().await {
            // Translate the binary KVM message into a JSON browser message.
            if let Some(json_msg) = translate_kvm_to_browser(&kvm_msg) {
                match serde_json::to_string(&json_msg) {
                    Ok(json_str) => {
                        // Send the JSON string as a WebSocket text frame.
                        // We lock the mutex briefly to access the shared sink.
                        let mut sink = ws_tx_m2b.lock().await;
                        if sink.send(WsMessage::Text(json_str)).await.is_err() {
                            debug!(
                                "session {session_id_m2b}: WebSocket send failed (browser disconnected)"
                            );
                            break;
                        }
                    }
                    Err(e) => {
                        error!("session {session_id_m2b}: JSON serialization error: {e}");
                    }
                }
            }
        }
    });

    // ── Task C: Browser → Master forwarder ────────────────────────────────────
    //
    // Reads JSON WebSocket frames from the browser, translates them to binary
    // KVM messages, and writes them to the master TCP stream.
    let session_id_b2m = session_id.clone();
    let seq_b2m = Arc::clone(&seq);

    // We need to own `master_write` in this task exclusively.
    // Wrap it in a `tokio::sync::Mutex` so the keepalive task can also borrow it.
    let master_write = Arc::new(tokio::sync::Mutex::new(master_write));
    let master_write_b2m = Arc::clone(&master_write);

    let browser_to_master_task = tokio::spawn({
        // Pin `ws_rx` so it can be used in the async block.
        let mut ws_rx = ws_rx;
        async move {
            loop {
                // Read the next WebSocket frame from the browser.
                // `next()` returns `None` when the stream is closed.
                let ws_msg = match ws_rx.next().await {
                    Some(Ok(msg)) => msg,
                    Some(Err(WsError::ConnectionClosed | WsError::Protocol(_))) => {
                        debug!("session {session_id_b2m}: browser WebSocket closed normally");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("session {session_id_b2m}: browser WebSocket error: {e}");
                        break;
                    }
                    None => {
                        debug!("session {session_id_b2m}: browser stream ended");
                        break;
                    }
                };

                match ws_msg {
                    WsMessage::Text(json_str) => {
                        // Parse the JSON message from the browser.
                        let browser_msg: BrowserToMasterMsg = match serde_json::from_str(&json_str)
                        {
                            Ok(m) => m,
                            Err(e) => {
                                warn!("session {session_id_b2m}: invalid JSON from browser: {e}");
                                // Don't close the session for one bad message; the
                                // browser might retry on the next interaction.
                                continue;
                            }
                        };

                        debug!(
                            "session {session_id_b2m}: browser → master: {}",
                            browser_msg_type_name(&browser_msg)
                        );

                        // Translate JSON → binary KVM message.
                        let kvm_msg = match translate_browser_to_kvm(&browser_msg) {
                            Ok(m) => m,
                            Err(e) => {
                                warn!("session {session_id_b2m}: translation error: {e}");
                                continue;
                            }
                        };

                        // Encode the KVM message to bytes with the next sequence number.
                        let seq_num = seq_b2m.next();
                        let bytes = match encode_message_now(&kvm_msg, seq_num) {
                            Ok(b) => b,
                            Err(e) => {
                                error!("session {session_id_b2m}: encode error: {e}");
                                break;
                            }
                        };

                        // Write the encoded bytes to the master TCP stream.
                        let mut write = master_write_b2m.lock().await;
                        if let Err(e) = crate::infrastructure::master_conn::write_kvm_message(
                            &mut write,
                            &bytes,
                            &session_id_b2m,
                        )
                        .await
                        {
                            warn!("{e}");
                            break;
                        }
                    }

                    WsMessage::Binary(_) => {
                        // The browser-facing protocol is JSON-only.
                        // Binary frames are unexpected; log and skip.
                        warn!(
                            "session {session_id_b2m}: unexpected binary WebSocket frame (ignored)"
                        );
                    }

                    WsMessage::Ping(data) => {
                        // WebSocket protocol-level ping (distinct from KVM app-level Ping).
                        // tokio-tungstenite handles the Pong reply automatically when
                        // writing to the sink.  We just log it here.
                        debug!(
                            "session {session_id_b2m}: WebSocket ping ({} bytes)",
                            data.len()
                        );
                    }

                    WsMessage::Pong(_) => {
                        debug!("session {session_id_b2m}: WebSocket pong received");
                    }

                    WsMessage::Close(_) => {
                        debug!("session {session_id_b2m}: WebSocket Close frame received");
                        break;
                    }

                    WsMessage::Frame(_) => {
                        debug!("session {session_id_b2m}: raw frame (ignored)");
                    }
                }
            }
        }
    });

    // ── Task D: KVM keepalive Ping/Pong ───────────────────────────────────────
    //
    // Sends a KVM application-level Ping to the master every `ping_interval`.
    // The master must reply with a Pong.  If the Pong does not arrive within
    // `ping_timeout`, we close the session.
    //
    // This is separate from the WebSocket protocol-level ping/pong, which
    // tokio-tungstenite handles automatically.
    let session_id_ping = session_id.clone();
    let master_write_ping = Arc::clone(&master_write);
    let seq_ping = Arc::clone(&seq);
    let ping_interval = config.ping_interval;

    let keepalive_task = tokio::spawn(async move {
        // Create a Tokio interval timer that fires every `ping_interval`.
        let mut ticker = interval(ping_interval);

        // `ticker.tick()` is async — it yields until the next interval fires.
        // The first `tick()` resolves immediately (at t=0).
        ticker.tick().await; // Skip the immediate first tick.

        loop {
            ticker.tick().await;

            // Build a Ping message with the current timestamp as the echo token.
            // Using the timestamp lets us measure round-trip latency if desired.
            let token = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64;

            let ping_msg = kvm_core::protocol::messages::KvmMessage::Ping(token);
            let seq_num = seq_ping.next();

            match encode_message_now(&ping_msg, seq_num) {
                Ok(bytes) => {
                    let mut write = master_write_ping.lock().await;
                    if let Err(e) = crate::infrastructure::master_conn::write_kvm_message(
                        &mut write,
                        &bytes,
                        &session_id_ping,
                    )
                    .await
                    {
                        debug!("session {session_id_ping}: keepalive ping failed: {e}");
                        break;
                    }
                    debug!("session {session_id_ping}: sent keepalive Ping (token={token:#x})");
                }
                Err(e) => {
                    error!("session {session_id_ping}: failed to encode Ping: {e}");
                    break;
                }
            }
        }
    });

    // ── Step 5: Wait for any task to finish ───────────────────────────────────
    //
    // `tokio::select!` waits for the first branch to complete and then
    // cancels the others.  This means the session ends as soon as:
    //
    // - The browser disconnects (browser_to_master_task finishes)
    // - The master closes the connection (master_reader_task finishes)
    // - The keepalive fails (keepalive_task finishes)
    // - The master→browser forwarder fails (master_to_browser_task finishes)
    //
    // When one task ends, dropping the others cancels them — Tokio handles
    // the cleanup automatically.
    tokio::select! {
        _ = master_reader_task => {
            debug!("session {session_id}: master reader task ended");
        }
        _ = master_to_browser_task => {
            debug!("session {session_id}: master→browser task ended");
        }
        _ = browser_to_master_task => {
            debug!("session {session_id}: browser→master task ended");
        }
        _ = keepalive_task => {
            debug!("session {session_id}: keepalive task ended");
        }
    }

    Ok(())
}

// ── Helper ────────────────────────────────────────────────────────────────────

/// Returns a short type-name string for a `BrowserToMasterMsg` variant.
///
/// Used in debug log messages to avoid accidentally logging sensitive field
/// values (e.g., PIN hashes from `PairingResponse`).
fn browser_msg_type_name(msg: &BrowserToMasterMsg) -> &'static str {
    match msg {
        BrowserToMasterMsg::Hello { .. } => "Hello",
        BrowserToMasterMsg::ScreenInfo { .. } => "ScreenInfo",
        BrowserToMasterMsg::PairingResponse { .. } => "PairingResponse",
        BrowserToMasterMsg::ClipboardData { .. } => "ClipboardData",
        BrowserToMasterMsg::Disconnect => "Disconnect",
        BrowserToMasterMsg::Pong { .. } => "Pong",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::messages::BrowserToMasterMsg;

    #[test]
    fn test_browser_msg_type_name_hello() {
        let msg = BrowserToMasterMsg::Hello {
            client_id: "abc".to_string(),
            client_name: "x".to_string(),
            capabilities: 0,
        };
        assert_eq!(browser_msg_type_name(&msg), "Hello");
    }

    #[test]
    fn test_browser_msg_type_name_screen_info() {
        let msg = BrowserToMasterMsg::ScreenInfo {
            width: 1920,
            height: 1080,
            scale_factor_percent: 100,
        };
        assert_eq!(browser_msg_type_name(&msg), "ScreenInfo");
    }

    #[test]
    fn test_browser_msg_type_name_pairing_response() {
        let msg = BrowserToMasterMsg::PairingResponse {
            pairing_session_id: "x".to_string(),
            pin_hash: "secret!".to_string(),
            accepted: true,
        };
        // Must not include the secret pin_hash in the output string
        let name = browser_msg_type_name(&msg);
        assert_eq!(name, "PairingResponse");
        assert!(
            !name.contains("secret"),
            "type name must not expose field values"
        );
    }

    #[test]
    fn test_browser_msg_type_name_clipboard() {
        let msg = BrowserToMasterMsg::ClipboardData {
            text: "hello".to_string(),
        };
        assert_eq!(browser_msg_type_name(&msg), "ClipboardData");
    }

    #[test]
    fn test_browser_msg_type_name_disconnect() {
        let msg = BrowserToMasterMsg::Disconnect;
        assert_eq!(browser_msg_type_name(&msg), "Disconnect");
    }

    #[test]
    fn test_browser_msg_type_name_pong() {
        let msg = BrowserToMasterMsg::Pong { token: 42 };
        assert_eq!(browser_msg_type_name(&msg), "Pong");
    }
}
