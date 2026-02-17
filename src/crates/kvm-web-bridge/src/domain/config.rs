//! Bridge configuration types.
//!
//! [`BridgeConfig`] is the single source of truth for all runtime settings.
//! It can be constructed from CLI arguments (preferred for production) or from
//! sensible defaults (useful for local development and tests).
//!
//! # Design rationale
//!
//! Keeping configuration as a plain struct (no global state, no environment
//! variable reads inside the domain) makes the bridge easy to embed in tests
//! and future orchestration systems.  The infrastructure layer is responsible
//! for populating the struct from CLI args or environment variables.

use std::net::SocketAddr;
use std::time::Duration;

/// All runtime configuration for the WebSocket bridge.
///
/// Build this struct once at startup (via CLI args or defaults) and then wrap
/// it in an `Arc` so it can be shared cheaply across all session tasks.
///
/// # Example
///
/// ```rust
/// use kvm_web_bridge::domain::BridgeConfig;
///
/// // Defaults are suitable for local development:
/// let cfg = BridgeConfig::default();
/// assert_eq!(cfg.ws_bind_addr.port(), 24803);
/// ```
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// The address and port the WebSocket server binds to.
    ///
    /// `0.0.0.0` accepts connections from any network interface (LAN +
    /// localhost).  Set to `127.0.0.1` to accept only local connections for
    /// additional security in production deployments.
    pub ws_bind_addr: SocketAddr,

    /// The TCP address of the KVM master's control port.
    ///
    /// When the bridge and master run on the same machine (single-host dev),
    /// this is `127.0.0.1:24800`.  In multi-host deployments, set this to
    /// the master's LAN IP address.
    pub master_addr: SocketAddr,

    /// How often to send a KVM application-level Ping to the master.
    ///
    /// This is separate from the WebSocket protocol-level ping/pong (which
    /// tokio-tungstenite handles automatically).  The KVM Ping keeps the
    /// master's session alive and lets the bridge detect a silent TCP failure.
    pub ping_interval: Duration,

    /// Maximum time to wait for a KVM Pong reply before the bridge considers
    /// the master connection dead and closes the session.
    pub ping_timeout: Duration,
}

impl Default for BridgeConfig {
    /// Returns a `BridgeConfig` suitable for local development without any
    /// external configuration.
    ///
    /// | Field           | Default             |
    /// |-----------------|---------------------|
    /// | ws_bind_addr    | `0.0.0.0:24803`     |
    /// | master_addr     | `127.0.0.1:24800`   |
    /// | ping_interval   | 5 seconds           |
    /// | ping_timeout    | 15 seconds          |
    fn default() -> Self {
        Self {
            // The `.parse().unwrap()` calls here are safe because these are
            // compile-time-known valid socket address strings.
            ws_bind_addr: "0.0.0.0:24803".parse().unwrap(),
            master_addr: "127.0.0.1:24800".parse().unwrap(),
            ping_interval: Duration::from_secs(5),
            ping_timeout: Duration::from_secs(15),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ws_port_is_24803() {
        // Arrange / Act
        let cfg = BridgeConfig::default();
        // Assert
        assert_eq!(cfg.ws_bind_addr.port(), 24803);
    }

    #[test]
    fn test_default_master_port_is_24800() {
        // Arrange / Act
        let cfg = BridgeConfig::default();
        // Assert
        assert_eq!(cfg.master_addr.port(), 24800);
    }

    #[test]
    fn test_default_master_ip_is_loopback() {
        let cfg = BridgeConfig::default();
        // The master defaults to localhost so the bridge can run on the same machine.
        assert_eq!(cfg.master_addr.ip().to_string(), "127.0.0.1");
    }

    #[test]
    fn test_default_ping_interval_is_5s() {
        let cfg = BridgeConfig::default();
        assert_eq!(cfg.ping_interval, Duration::from_secs(5));
    }

    #[test]
    fn test_default_ping_timeout_is_15s() {
        let cfg = BridgeConfig::default();
        assert_eq!(cfg.ping_timeout, Duration::from_secs(15));
    }

    #[test]
    fn test_config_can_be_cloned() {
        // Cloneability is required so an Arc<BridgeConfig> can be shared
        // across session tasks.
        let cfg = BridgeConfig::default();
        let cloned = cfg.clone();
        assert_eq!(cfg.ws_bind_addr, cloned.ws_bind_addr);
        assert_eq!(cfg.master_addr, cloned.master_addr);
    }

    #[test]
    fn test_config_custom_addresses() {
        // Verify that custom addresses are stored correctly.
        let cfg = BridgeConfig {
            ws_bind_addr: "127.0.0.1:9000".parse().unwrap(),
            master_addr: "10.0.0.5:24800".parse().unwrap(),
            ping_interval: Duration::from_secs(10),
            ping_timeout: Duration::from_secs(30),
        };
        assert_eq!(cfg.ws_bind_addr.port(), 9000);
        assert_eq!(cfg.master_addr.ip().to_string(), "10.0.0.5");
        assert_eq!(cfg.ping_interval, Duration::from_secs(10));
        assert_eq!(cfg.ping_timeout, Duration::from_secs(30));
    }
}
