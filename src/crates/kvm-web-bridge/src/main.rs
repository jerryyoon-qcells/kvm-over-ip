//! KVM-Over-IP WebSocket Bridge.
//!
//! Bridges browser WebSocket clients (WSS) to the native KVM protocol so that
//! web-based clients can participate in the KVM session without requiring raw
//! TCP socket support in the browser.
//!
//! # Why a WebSocket bridge? (for beginners)
//!
//! Browsers cannot open raw TCP connections — they can only use WebSockets
//! (RFC 6455), which start with an HTTP upgrade handshake and then carry
//! framed binary or text messages.
//!
//! The native `kvm-client` binary uses a raw TCP control channel to the master.
//! To support browser-based clients (e.g., a Chromebook or tablet that cannot
//! run a native app), `kvm-web-bridge` acts as a proxy:
//!
//! ```text
//! Browser (WebSocket) ──> kvm-web-bridge (TCP) ──> kvm-master (TCP)
//! ```
//!
//! The bridge accepts WebSocket connections on port 24803 and forwards the
//! raw bytes to the master's TCP control port (24800).  Because the KVM wire
//! format is binary-safe (a fixed-size header + payload), the bridge can
//! forward bytes without parsing them.
//!
//! # Connection model
//!
//! Each browser tab that connects to the bridge gets its own pair of TCP
//! sockets (one to the browser, one to the master).  The bridge spawns a
//! Tokio task per connection that runs a bidirectional byte copy:
//!
//! ```text
//! browser → master:  tokio::io::copy(ws_read, master_write)
//! master → browser:  tokio::io::copy(master_read, ws_write)
//! ```
//!
//! When either side closes its connection, the `tokio::select!` macro detects
//! it and exits the task, which closes both sockets.
//!
//! # Configuration
//!
//! The bridge is configured via environment variables (no config file needed):
//!
//! | Variable          | Default             | Description                        |
//! |-------------------|---------------------|------------------------------------|
//! | `KVM_WS_PORT`     | `24803`             | WebSocket listener port            |
//! | `KVM_MASTER_ADDR` | `127.0.0.1:24800`   | Master TCP address                 |
//!
//! # Graceful shutdown
//!
//! A Ctrl+C signal handler (via `tokio::signal::ctrl_c`) sets an `AtomicBool`
//! to `false`.  The accept loop checks this flag every 200 ms (using a
//! `tokio::time::timeout` on the `accept` call) and exits cleanly when the
//! flag is cleared.
//!
//! # Production note
//!
//! This implementation does **not** perform a full WebSocket handshake (RFC 6455
//! upgrade, frame masking/unmasking).  It is a raw TCP proxy that works correctly
//! when the browser uses a WebSocket library that is compatible with binary
//! pass-through, or in the initial integration phase.  A production build should
//! use `tokio-tungstenite` for full WebSocket support.

use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Context;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

// ── Configuration ─────────────────────────────────────────────────────────────

/// Command-line / environment-driven configuration.
///
/// Built from environment variables at startup via [`BridgeConfig::from_env`].
/// All fields have sensible defaults so the bridge can be started without any
/// configuration for local development.
#[derive(Debug)]
struct BridgeConfig {
    /// Address + port the WebSocket listener binds to.
    ///
    /// `0.0.0.0` means "listen on all network interfaces", which allows
    /// browsers on the local network to reach the bridge.
    ws_bind_addr: SocketAddr,
    /// Address of the master's TCP control port.
    ///
    /// In a typical deployment this is `127.0.0.1:24800` when the bridge
    /// runs on the same machine as the master.
    master_addr: SocketAddr,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            ws_bind_addr: "0.0.0.0:24803".parse().unwrap(),
            master_addr: "127.0.0.1:24800".parse().unwrap(),
        }
    }
}

impl BridgeConfig {
    /// Builds config from environment variables, falling back to defaults.
    ///
    /// Invalid values (non-numeric port, malformed address) are silently ignored
    /// and the default is used instead.  This is intentional: misconfiguration
    /// should not prevent the bridge from starting with safe defaults.
    fn from_env() -> Self {
        let mut cfg = Self::default();

        if let Ok(val) = std::env::var("KVM_WS_PORT") {
            if let Ok(port) = val.parse::<u16>() {
                cfg.ws_bind_addr =
                    format!("0.0.0.0:{port}").parse().unwrap_or(cfg.ws_bind_addr);
            }
        }

        if let Ok(val) = std::env::var("KVM_MASTER_ADDR") {
            if let Ok(addr) = val.parse::<SocketAddr>() {
                cfg.master_addr = addr;
            }
        }

        cfg
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Entry point for the WebSocket bridge process.
///
/// 1. Initialises structured logging (`tracing`).
/// 2. Loads configuration from environment variables.
/// 3. Binds a TCP listener on the WebSocket port.
/// 4. Starts a Ctrl+C handler that sets `running = false`.
/// 5. Accept loop: for each incoming connection, spawn a Tokio task that
///    proxies bytes between the browser and the master.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise the `tracing` subscriber.  The log level is read from the
    // `RUST_LOG` environment variable (e.g., `RUST_LOG=debug`); defaults to
    // `info` if the variable is absent or invalid.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = BridgeConfig::from_env();
    info!(
        "KVM-Over-IP WebSocket bridge starting (ws={}, master={})",
        cfg.ws_bind_addr, cfg.master_addr
    );

    // `running` is an AtomicBool shared between the main loop and the Ctrl+C
    // handler task.  Using an atomic avoids the need for a Mutex.
    let running = Arc::new(AtomicBool::new(true));

    let listener = tokio::net::TcpListener::bind(cfg.ws_bind_addr)
        .await
        .with_context(|| format!("failed to bind WebSocket listener on {}", cfg.ws_bind_addr))?;

    info!("WebSocket bridge listening on {}", cfg.ws_bind_addr);

    // Spawn the Ctrl+C handler in a background task.  When the signal arrives,
    // it sets `running` to false; the main accept loop will notice on its next
    // iteration and exit cleanly.
    let running_clone = Arc::clone(&running);
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("shutdown signal received");
            running_clone.store(false, Ordering::Relaxed);
        }
    });

    loop {
        // Exit the accept loop if the running flag has been cleared.
        if !running.load(Ordering::Relaxed) {
            break;
        }

        // Use a 200 ms timeout on `accept` so the loop can check `running`
        // even when no clients are connecting.  Without a timeout the loop
        // would block indefinitely on `accept` after Ctrl+C.
        let accept_result = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            listener.accept(),
        )
        .await;

        match accept_result {
            Ok(Ok((stream, peer_addr))) => {
                info!("WebSocket client accepted: {peer_addr}");
                let master_addr = cfg.master_addr;
                // Spawn a per-connection task so that one slow browser client
                // does not block new connections from being accepted.
                tokio::spawn(async move {
                    if let Err(e) = handle_ws_client(stream, peer_addr, master_addr).await {
                        warn!("WebSocket client {peer_addr} error: {e}");
                    }
                });
            }
            Ok(Err(e)) => {
                error!("accept error: {e}");
            }
            Err(_) => {
                // Timeout expired — no new connection.  Loop back to check
                // the `running` flag.
            }
        }
    }

    info!("KVM-Over-IP WebSocket bridge stopped");
    Ok(())
}

/// Handles a single WebSocket client by proxying raw bytes between the
/// browser TCP stream and the master's control channel.
///
/// Opens a TCP connection to the master, then runs two concurrent byte-copy
/// tasks using `tokio::select!`:
/// - browser → master
/// - master → browser
///
/// When either side closes the connection, `select!` returns and both
/// sockets are dropped (which closes them).
///
/// NOTE: Full WebSocket framing (RFC 6455 handshake and frame codec) is not
/// implemented here; this stub connects the raw TCP sockets.  A production
/// implementation would use the `tokio-tungstenite` crate to properly handle
/// the WebSocket upgrade handshake and frame masking/unmasking.
async fn handle_ws_client(
    ws_stream: tokio::net::TcpStream,
    peer_addr: SocketAddr,
    master_addr: SocketAddr,
) -> anyhow::Result<()> {
    // Connect to the master's TCP control port.
    let master_stream = tokio::net::TcpStream::connect(master_addr)
        .await
        .with_context(|| format!("bridge: failed to connect to master at {master_addr}"))?;

    info!("bridge: {peer_addr} <-> {master_addr}");

    // Split both streams into independent read and write halves so that the
    // two copy directions can run concurrently without borrowing the same stream.
    let (mut ws_read, mut ws_write) = tokio::io::split(ws_stream);
    let (mut master_read, mut master_write) = tokio::io::split(master_stream);

    // `tokio::io::copy` reads from the source and writes to the destination
    // in a loop until EOF or an error.
    let browser_to_master =
        tokio::io::copy(&mut ws_read, &mut master_write);
    let master_to_browser =
        tokio::io::copy(&mut master_read, &mut ws_write);

    // Run both halves concurrently with `select!`.  The first branch to
    // complete (i.e., whichever side disconnects first) causes both halves
    // to be dropped, closing the remaining connection.
    tokio::select! {
        result = browser_to_master => {
            if let Err(e) = result {
                warn!("browser→master copy error for {peer_addr}: {e}");
            }
        }
        result = master_to_browser => {
            if let Err(e) = result {
                warn!("master→browser copy error for {peer_addr}: {e}");
            }
        }
    }

    info!("bridge: connection closed for {peer_addr}");
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_config_default_ws_port_is_24803() {
        // Arrange / Act
        let cfg = BridgeConfig::default();

        // Assert
        assert_eq!(cfg.ws_bind_addr.port(), 24803);
    }

    #[test]
    fn test_bridge_config_default_master_port_is_24800() {
        let cfg = BridgeConfig::default();
        assert_eq!(cfg.master_addr.port(), 24800);
    }

    #[test]
    fn test_bridge_config_from_env_uses_defaults_when_vars_absent() {
        // Remove env vars to ensure clean state
        std::env::remove_var("KVM_WS_PORT");
        std::env::remove_var("KVM_MASTER_ADDR");

        let cfg = BridgeConfig::from_env();

        assert_eq!(cfg.ws_bind_addr.port(), 24803);
        assert_eq!(cfg.master_addr.port(), 24800);
    }

    #[test]
    fn test_bridge_config_from_env_reads_ws_port() {
        // Arrange
        std::env::set_var("KVM_WS_PORT", "9999");

        // Act
        let cfg = BridgeConfig::from_env();

        // Assert
        assert_eq!(cfg.ws_bind_addr.port(), 9999);

        // Cleanup
        std::env::remove_var("KVM_WS_PORT");
    }

    #[test]
    fn test_bridge_config_from_env_reads_master_addr() {
        // Arrange
        std::env::set_var("KVM_MASTER_ADDR", "10.0.0.5:8080");

        // Act
        let cfg = BridgeConfig::from_env();

        // Assert
        assert_eq!(cfg.master_addr.to_string(), "10.0.0.5:8080");

        // Cleanup
        std::env::remove_var("KVM_MASTER_ADDR");
    }

    #[test]
    fn test_bridge_config_ignores_invalid_port() {
        // Arrange
        std::env::set_var("KVM_WS_PORT", "not_a_number");

        // Act
        let cfg = BridgeConfig::from_env();

        // Assert – fallback to default when the value is invalid
        assert_eq!(cfg.ws_bind_addr.port(), 24803);

        // Cleanup
        std::env::remove_var("KVM_WS_PORT");
    }
}
