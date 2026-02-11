//! KVM-Over-IP WebSocket Bridge.
//!
//! Bridges browser WebSocket clients (WSS) to the native KVM protocol so that
//! web-based clients can participate in the KVM session without requiring raw
//! UDP socket support.
//!
//! # Protocol Bridge
//!
//! ```text
//! Browser (WSS) <──> kvm-web-bridge <──> kvm-master (TCP control channel)
//! ```
//!
//! Each WebSocket connection from a browser tab is proxied to the master's TCP
//! control channel.  Binary frames are forwarded verbatim; the KVM wire format
//! is preserved end-to-end.
//!
//! # Usage
//!
//! ```
//! kvm-web-bridge --ws-port 24803 --master-addr 127.0.0.1:24800
//! ```

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
#[derive(Debug)]
struct BridgeConfig {
    /// Address + port the WebSocket listener binds to.
    ws_bind_addr: SocketAddr,
    /// Address of the master's TCP control port.
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    let running = Arc::new(AtomicBool::new(true));
    let listener = tokio::net::TcpListener::bind(cfg.ws_bind_addr)
        .await
        .with_context(|| format!("failed to bind WebSocket listener on {}", cfg.ws_bind_addr))?;

    info!("WebSocket bridge listening on {}", cfg.ws_bind_addr);

    let running_clone = Arc::clone(&running);
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("shutdown signal received");
            running_clone.store(false, Ordering::Relaxed);
        }
    });

    loop {
        if !running.load(Ordering::Relaxed) {
            break;
        }

        let accept_result = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            listener.accept(),
        )
        .await;

        match accept_result {
            Ok(Ok((stream, peer_addr))) => {
                info!("WebSocket client accepted: {peer_addr}");
                let master_addr = cfg.master_addr;
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
                // Timeout – check running flag.
            }
        }
    }

    info!("KVM-Over-IP WebSocket bridge stopped");
    Ok(())
}

/// Handles a single WebSocket client by proxying raw bytes between the
/// browser TCP stream and the master's control channel.
///
/// NOTE: Full WebSocket framing (RFC 6455 handshake and frame codec) is not
/// implemented here; this stub connects the raw TCP sockets.  A production
/// implementation would use the `tokio-tungstenite` crate.
async fn handle_ws_client(
    ws_stream: tokio::net::TcpStream,
    peer_addr: SocketAddr,
    master_addr: SocketAddr,
) -> anyhow::Result<()> {
    // Connect to master.
    let master_stream = tokio::net::TcpStream::connect(master_addr)
        .await
        .with_context(|| format!("bridge: failed to connect to master at {master_addr}"))?;

    info!("bridge: {peer_addr} <-> {master_addr}");

    let (mut ws_read, mut ws_write) = tokio::io::split(ws_stream);
    let (mut master_read, mut master_write) = tokio::io::split(master_stream);

    // Bidirectional copy: browser → master and master → browser.
    let browser_to_master =
        tokio::io::copy(&mut ws_read, &mut master_write);
    let master_to_browser =
        tokio::io::copy(&mut master_read, &mut ws_write);

    // Run both halves concurrently; exit when either side closes.
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
