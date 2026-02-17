//! KVM-Over-IP WebSocket Bridge — entry point.
//!
//! This binary accepts WebSocket connections from web browsers and proxies them
//! to the KVM master's native TCP control channel.  It acts as a thin
//! translation layer between the JSON-over-WebSocket browser protocol and the
//! compact binary KVM protocol.
//!
//! # Why a separate bridge process?
//!
//! Web browsers can only communicate over HTTP/WebSocket — they cannot open raw
//! TCP sockets.  The native KVM protocol uses a custom binary framing on top of
//! raw TCP.  This bridge translates between the two so the web client (e.g., a
//! React app) can:
//!
//! - Send JSON-encoded control messages to the master.
//! - Receive JSON-encoded input events from the master and inject them into the
//!   browser DOM.
//!
//! # Usage
//!
//! ```text
//! kvm-web-bridge [OPTIONS]
//!
//! Options:
//!   --ws-port     <PORT>   WebSocket listener port [default: 24803]
//!   --master-host <HOST>   KVM master hostname or IP [default: 127.0.0.1]
//!   --master-port <PORT>   KVM master control port [default: 24800]
//!   --ping-interval <SECS> Keepalive ping interval in seconds [default: 5]
//!   --ping-timeout  <SECS> Ping timeout in seconds [default: 15]
//! ```
//!
//! # Environment variable overrides
//!
//! The CLI defaults can also be overridden with environment variables.
//! CLI args take precedence when both are present.
//!
//! | Variable             | Default           | Description                    |
//! |----------------------|-------------------|--------------------------------|
//! | `KVM_WS_PORT`        | `24803`           | WebSocket listener port        |
//! | `KVM_MASTER_ADDR`    | `127.0.0.1:24800` | Master TCP address             |
//! | `KVM_PING_INTERVAL`  | `5`               | Keepalive ping interval (secs) |
//! | `KVM_PING_TIMEOUT`   | `15`              | Ping timeout (secs)            |
//!
//! # Architecture overview
//!
//! ```text
//! Web Browser  (JSON over WebSocket)
//!       ↕
//! kvm-web-bridge  ← this process
//!   domain/       JSON message types, BridgeConfig
//!   application/  Translate JSON ↔ binary KVM
//!   infrastructure/
//!     ws_server/  Accept WebSocket connections
//!     master_conn/ TCP connection to kvm-master
//!       ↕
//! kvm-master  (binary KVM protocol over TCP, port 24800)
//! ```

use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

// Import the domain config and the infrastructure server runner from our
// library crate (`kvm_web_bridge`).
use kvm_web_bridge::domain::BridgeConfig;
use kvm_web_bridge::infrastructure::run_server;

// ── CLI argument definitions ──────────────────────────────────────────────────

/// KVM-Over-IP WebSocket bridge.
///
/// Accepts WebSocket connections from browsers and proxies them to the KVM
/// master's native TCP control channel.
///
/// The `#[derive(Parser)]` macro from `clap` generates the argument parser
/// automatically from the struct fields and their `#[arg(...)]` attributes.
#[derive(Debug, Parser)]
#[command(
    name = "kvm-web-bridge",
    about = "WebSocket-to-native-protocol bridge for KVM-Over-IP web clients",
    version
)]
struct Cli {
    /// TCP port for the WebSocket server to listen on.
    ///
    /// Browsers connect to this port via WebSocket (ws://host:PORT).
    #[arg(long, default_value_t = 24803, env = "KVM_WS_PORT")]
    ws_port: u16,

    /// IP address to bind the WebSocket server to.
    ///
    /// Use `0.0.0.0` to accept connections from any network interface (LAN +
    /// localhost), or `127.0.0.1` to accept only local connections.
    #[arg(long, default_value = "0.0.0.0", env = "KVM_WS_BIND")]
    ws_bind: String,

    /// Hostname or IP address of the KVM master.
    ///
    /// When the bridge and master run on the same machine, use `127.0.0.1`.
    /// In multi-host deployments, set this to the master's LAN IP address.
    #[arg(long, default_value = "127.0.0.1", env = "KVM_MASTER_HOST")]
    master_host: String,

    /// TCP port of the KVM master's control channel.
    #[arg(long, default_value_t = 24800, env = "KVM_MASTER_PORT")]
    master_port: u16,

    /// Keepalive ping interval in seconds.
    ///
    /// The bridge sends a KVM application-level Ping to the master every this
    /// many seconds.  If the master does not reply within `--ping-timeout`
    /// seconds, the session is closed.
    #[arg(long, default_value_t = 5, env = "KVM_PING_INTERVAL")]
    ping_interval: u64,

    /// Keepalive ping timeout in seconds.
    ///
    /// If the master does not reply to a Ping within this many seconds, the
    /// bridge considers the connection dead and closes the session.
    #[arg(long, default_value_t = 15, env = "KVM_PING_TIMEOUT")]
    ping_timeout: u64,
}

impl Cli {
    /// Converts the parsed CLI arguments into a [`BridgeConfig`].
    ///
    /// # Errors
    ///
    /// Returns an error if `--ws-bind` or `--master-host` is not a valid IP
    /// address, or if the resulting socket address string cannot be parsed.
    fn into_bridge_config(self) -> anyhow::Result<BridgeConfig> {
        // Construct the WebSocket bind address from --ws-bind and --ws-port.
        let ws_bind_addr: SocketAddr = format!("{}:{}", self.ws_bind, self.ws_port)
            .parse()
            .with_context(|| {
                format!(
                    "invalid WebSocket bind address: '{}:{}'",
                    self.ws_bind, self.ws_port
                )
            })?;

        // Construct the master address from --master-host and --master-port.
        let master_addr: SocketAddr = format!("{}:{}", self.master_host, self.master_port)
            .parse()
            .with_context(|| {
                format!(
                    "invalid master address: '{}:{}'",
                    self.master_host, self.master_port
                )
            })?;

        Ok(BridgeConfig {
            ws_bind_addr,
            master_addr,
            ping_interval: Duration::from_secs(self.ping_interval),
            ping_timeout: Duration::from_secs(self.ping_timeout),
        })
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Program entry point.
///
/// The `#[tokio::main]` attribute sets up the Tokio multi-threaded async
/// runtime.  All async tasks (WebSocket sessions, keepalive loops, etc.) run
/// on this runtime's thread pool.
///
/// # What happens at startup
///
/// 1. `tracing_subscriber` is initialised to format log output.  The log
///    level is controlled by the `RUST_LOG` environment variable (e.g.,
///    `RUST_LOG=debug`).
/// 2. CLI arguments are parsed with `clap` into a [`Cli`] struct.
/// 3. A [`BridgeConfig`] is constructed from the CLI arguments.
/// 4. A Ctrl+C handler is spawned; it sets a shared `AtomicBool` to `false`
///    when the user presses Ctrl+C.
/// 5. [`run_server`] is called, which binds the WebSocket port and accepts
///    browser connections until the shutdown flag is cleared.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Logging setup ─────────────────────────────────────────────────────────
    //
    // `EnvFilter::try_from_default_env()` reads the `RUST_LOG` environment
    // variable.  If it is absent or invalid, we fall back to `info` level.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // ── Parse CLI arguments ───────────────────────────────────────────────────
    //
    // `Cli::parse()` reads from `std::env::args()` and exits with a usage
    // message if required arguments are missing or values are invalid.
    let cli = Cli::parse();

    // Convert the CLI arguments into a BridgeConfig.
    let config = cli.into_bridge_config()?;

    info!(
        "KVM-Over-IP WebSocket bridge starting — ws={}, master={}",
        config.ws_bind_addr, config.master_addr
    );

    // ── Graceful shutdown flag ─────────────────────────────────────────────────
    //
    // `AtomicBool` is a thread-safe boolean that can be read and written from
    // multiple threads without a Mutex.  We use `Relaxed` ordering because we
    // only need the value to eventually propagate — precise ordering is not
    // required here.
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    // Spawn a task that listens for Ctrl+C (SIGINT on Unix).
    // When received, it sets `running` to false.  The accept loop in
    // `run_server` checks this flag every 200 ms and exits cleanly.
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("received Ctrl+C — initiating graceful shutdown");
                running_clone.store(false, Ordering::Relaxed);
            }
            Err(e) => {
                tracing::error!("failed to listen for Ctrl+C signal: {e}");
            }
        }
    });

    // ── Main server loop ───────────────────────────────────────────────────────
    run_server(config, running).await?;

    info!("KVM-Over-IP WebSocket bridge stopped");
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_defaults_produce_correct_ws_port() {
        // Arrange: parse with no arguments (all defaults apply)
        let cli = Cli::parse_from(["kvm-web-bridge"]);

        // Assert
        assert_eq!(cli.ws_port, 24803);
    }

    #[test]
    fn test_cli_defaults_produce_correct_master_port() {
        let cli = Cli::parse_from(["kvm-web-bridge"]);
        assert_eq!(cli.master_port, 24800);
    }

    #[test]
    fn test_cli_defaults_produce_correct_master_host() {
        let cli = Cli::parse_from(["kvm-web-bridge"]);
        assert_eq!(cli.master_host, "127.0.0.1");
    }

    #[test]
    fn test_cli_defaults_produce_correct_ping_interval() {
        let cli = Cli::parse_from(["kvm-web-bridge"]);
        assert_eq!(cli.ping_interval, 5);
    }

    #[test]
    fn test_cli_defaults_produce_correct_ping_timeout() {
        let cli = Cli::parse_from(["kvm-web-bridge"]);
        assert_eq!(cli.ping_timeout, 15);
    }

    #[test]
    fn test_cli_ws_port_override() {
        // Arrange: override --ws-port
        let cli = Cli::parse_from(["kvm-web-bridge", "--ws-port", "9999"]);

        // Assert
        assert_eq!(cli.ws_port, 9999);
    }

    #[test]
    fn test_cli_master_host_override() {
        let cli = Cli::parse_from(["kvm-web-bridge", "--master-host", "10.0.0.5"]);
        assert_eq!(cli.master_host, "10.0.0.5");
    }

    #[test]
    fn test_cli_master_port_override() {
        let cli = Cli::parse_from(["kvm-web-bridge", "--master-port", "9876"]);
        assert_eq!(cli.master_port, 9876);
    }

    #[test]
    fn test_cli_ping_interval_override() {
        let cli = Cli::parse_from(["kvm-web-bridge", "--ping-interval", "10"]);
        assert_eq!(cli.ping_interval, 10);
    }

    #[test]
    fn test_cli_ping_timeout_override() {
        let cli = Cli::parse_from(["kvm-web-bridge", "--ping-timeout", "30"]);
        assert_eq!(cli.ping_timeout, 30);
    }

    #[test]
    fn test_into_bridge_config_default_ws_port() {
        // Arrange: default CLI args
        let cli = Cli::parse_from(["kvm-web-bridge"]);

        // Act
        let config = cli.into_bridge_config().unwrap();

        // Assert
        assert_eq!(config.ws_bind_addr.port(), 24803);
    }

    #[test]
    fn test_into_bridge_config_default_master_port() {
        let cli = Cli::parse_from(["kvm-web-bridge"]);
        let config = cli.into_bridge_config().unwrap();
        assert_eq!(config.master_addr.port(), 24800);
    }

    #[test]
    fn test_into_bridge_config_default_master_ip() {
        let cli = Cli::parse_from(["kvm-web-bridge"]);
        let config = cli.into_bridge_config().unwrap();
        assert_eq!(config.master_addr.ip().to_string(), "127.0.0.1");
    }

    #[test]
    fn test_into_bridge_config_custom_ws_port() {
        let cli = Cli::parse_from(["kvm-web-bridge", "--ws-port", "8080"]);
        let config = cli.into_bridge_config().unwrap();
        assert_eq!(config.ws_bind_addr.port(), 8080);
    }

    #[test]
    fn test_into_bridge_config_custom_master_addr() {
        let cli = Cli::parse_from([
            "kvm-web-bridge",
            "--master-host",
            "192.168.1.100",
            "--master-port",
            "9000",
        ]);
        let config = cli.into_bridge_config().unwrap();
        assert_eq!(config.master_addr.to_string(), "192.168.1.100:9000");
    }

    #[test]
    fn test_into_bridge_config_ping_interval() {
        let cli = Cli::parse_from(["kvm-web-bridge", "--ping-interval", "10"]);
        let config = cli.into_bridge_config().unwrap();
        assert_eq!(config.ping_interval, Duration::from_secs(10));
    }

    #[test]
    fn test_into_bridge_config_ping_timeout() {
        let cli = Cli::parse_from(["kvm-web-bridge", "--ping-timeout", "30"]);
        let config = cli.into_bridge_config().unwrap();
        assert_eq!(config.ping_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_into_bridge_config_invalid_ws_bind_returns_error() {
        // Arrange: provide an invalid IP address string
        let cli = Cli {
            ws_port: 24803,
            ws_bind: "not.an.ip".to_string(),
            master_host: "127.0.0.1".to_string(),
            master_port: 24800,
            ping_interval: 5,
            ping_timeout: 15,
        };

        // Act
        let result = cli.into_bridge_config();

        // Assert: must return an error, not panic
        assert!(result.is_err());
    }

    #[test]
    fn test_into_bridge_config_invalid_master_host_returns_error() {
        // Arrange: provide an invalid master host
        let cli = Cli {
            ws_port: 24803,
            ws_bind: "0.0.0.0".to_string(),
            master_host: "not.an.ip".to_string(),
            master_port: 24800,
            ping_interval: 5,
            ping_timeout: 15,
        };

        // Act
        let result = cli.into_bridge_config();

        // Assert
        assert!(result.is_err());
    }
}
