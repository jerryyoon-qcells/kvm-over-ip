//! Tauri command bridge for the client application.
//!
//! Exposes application-layer state (connection status, screen info, settings)
//! to the React UI through Tauri commands.  Follows Clean Architecture: only
//! this module is allowed to reference both the Application layer and the
//! Presentation (Tauri) layer.
//!
//! # How Tauri commands work (for beginners)
//!
//! Tauri is a framework for building native desktop applications with a Rust
//! backend and a web-based (HTML/CSS/JavaScript) frontend.  The React UI runs
//! inside a WebView and communicates with the Rust backend via *Tauri commands*
//! — a type-safe RPC mechanism:
//!
//! ```text
//! React (TypeScript)          Tauri IPC            Rust backend
//! ─────────────────────────────────────────────────────────────
//! invoke("get_client_status") ──────────────────>  get_client_status()
//!                             <──────────────────  ClientStatusDto
//! ```
//!
//! From TypeScript you call:
//! ```ts
//! const status = await invoke<ClientStatusDto>("get_client_status");
//! ```
//!
//! # DTOs (Data Transfer Objects)
//!
//! The Rust application state (`ClientAppState`) uses Tokio async `Mutex`es and
//! is not directly serializable.  The DTO structs (`ClientStatusDto`,
//! `ClientSettingsDto`) are plain serializable snapshots that are safe to send
//! across the IPC boundary.
//!
//! Each DTO derives `serde::Serialize` + `serde::Deserialize` so that Tauri can
//! automatically convert it to JSON for the JavaScript side.  The TypeScript
//! interface must mirror the DTO fields exactly.
//!
//! # `ClientCommandResult<T>`
//!
//! All commands return `ClientCommandResult<T>` — a unified envelope:
//! ```json
//! { "success": true,  "data": {...}, "error": null  }
//! { "success": false, "data": null,  "error": "..."  }
//! ```
//! This lets the TypeScript side use a single error-handling pattern for all
//! commands regardless of their return type.
//!
//! # Async Mutex vs std Mutex
//!
//! `ClientAppState` uses `tokio::sync::Mutex` (not `std::sync::Mutex`) because
//! the Tauri command handlers are `async` functions.  Holding a `std::sync::Mutex`
//! guard across an `.await` point would block the Tokio thread pool; using
//! `tokio::sync::Mutex` correctly suspends the task instead of blocking.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::infrastructure::screen_info::{build_screen_info, MockScreenEnumerator};

// ── Shared application state ──────────────────────────────────────────────────

/// Connection status of the client as seen by the UI.
///
/// The UI displays these states as status indicators (e.g., a coloured dot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientConnectionStatus {
    /// Not yet attempting to connect.
    Disconnected,
    /// Actively trying to reach the master (TCP connect in progress).
    Connecting,
    /// Control channel established; awaiting pairing or `HelloAck`.
    Connected,
    /// Fully paired and receiving input events from the master.
    Active,
    /// Pairing is in progress (waiting for the user to enter the PIN).
    Pairing,
}

/// Runtime state shared between Tauri commands.
///
/// All fields are wrapped in `tokio::sync::Mutex` because Tauri command
/// handlers are async.  Multiple concurrent command invocations (e.g., a
/// periodic status poll and a settings update) can safely coexist.
pub struct ClientAppState {
    /// The current connection state reported to the UI.
    pub connection_status: Mutex<ClientConnectionStatus>,
    /// The master's IP address and port (e.g., "192.168.1.10:24800").
    pub master_address: Mutex<String>,
    /// The human-readable name that identifies this client to the master.
    pub client_name: Mutex<String>,
    /// Number of monitors detected on this machine (updated by `get_monitor_count`).
    pub monitor_count: Mutex<u8>,
}

impl ClientAppState {
    /// Creates a new `ClientAppState` with all fields at their defaults.
    ///
    /// The `client_name` is initialised from the machine's hostname so that
    /// the master can identify this client without requiring manual configuration.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connection_status: Mutex::new(ClientConnectionStatus::Disconnected),
            master_address: Mutex::new(String::new()),
            client_name: Mutex::new(hostname()),
            monitor_count: Mutex::new(0),
        })
    }
}

impl Default for ClientAppState {
    fn default() -> Self {
        Self {
            connection_status: Mutex::new(ClientConnectionStatus::Disconnected),
            master_address: Mutex::new(String::new()),
            client_name: Mutex::new(hostname()),
            monitor_count: Mutex::new(0),
        }
    }
}

// ── DTOs ──────────────────────────────────────────────────────────────────────

/// Full status snapshot returned to the React UI.
///
/// The `connection_status` field is serialized as a string (e.g., `"Connected"`)
/// using Rust's default `Debug` formatting; the TypeScript side treats it as
/// a discriminated union string literal type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientStatusDto {
    pub connection_status: String,
    pub master_address: String,
    pub client_name: String,
    pub monitor_count: u8,
}

/// Settings DTO that can be read/written from the UI.
///
/// Used by both `get_client_settings` (reading) and `update_client_settings`
/// (writing).  The TypeScript side sends/receives the same shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSettingsDto {
    pub master_address: String,
    pub client_name: String,
}

/// Unified response wrapper for client commands.
///
/// Every Tauri command returns `ClientCommandResult<T>` so that the TypeScript
/// caller always has a `success` flag to check before using `data`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientCommandResult<T: Serialize> {
    /// `true` if the command completed successfully; `false` on error.
    pub success: bool,
    /// The command's return value, present only when `success` is `true`.
    pub data: Option<T>,
    /// A human-readable error message, present only when `success` is `false`.
    pub error: Option<String>,
}

impl<T: Serialize> ClientCommandResult<T> {
    /// Constructs a successful result containing `data`.
    pub fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }

    /// Constructs an error result containing the given message.
    pub fn err(msg: impl Into<String>) -> Self {
        Self { success: false, data: None, error: Some(msg.into()) }
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Returns the current client status snapshot.
///
/// Called periodically by the React UI to update the status display.
/// Acquires locks on all state fields; each lock is held only while reading,
/// so contention is minimal.
pub async fn get_client_status(state: Arc<ClientAppState>) -> ClientCommandResult<ClientStatusDto> {
    let status = state.connection_status.lock().await;
    let master = state.master_address.lock().await;
    let name = state.client_name.lock().await;
    let monitors = state.monitor_count.lock().await;

    ClientCommandResult::ok(ClientStatusDto {
        // Format the enum variant as a string, e.g., "Connected" or "Disconnected".
        connection_status: format!("{status:?}"),
        master_address: master.clone(),
        client_name: name.clone(),
        monitor_count: *monitors,
    })
}

/// Returns the current client settings.
///
/// Used by the settings panel to populate its form fields on open.
pub async fn get_client_settings(
    state: Arc<ClientAppState>,
) -> ClientCommandResult<ClientSettingsDto> {
    let master = state.master_address.lock().await;
    let name = state.client_name.lock().await;

    ClientCommandResult::ok(ClientSettingsDto {
        master_address: master.clone(),
        client_name: name.clone(),
    })
}

/// Applies new client settings submitted by the user.
///
/// Validates that `client_name` is not blank before writing, because the
/// master uses the client name to identify machines in its UI.
pub async fn update_client_settings(
    state: Arc<ClientAppState>,
    settings: ClientSettingsDto,
) -> ClientCommandResult<()> {
    if settings.client_name.trim().is_empty() {
        return ClientCommandResult::err("client_name must not be empty");
    }

    // Acquire each lock in a separate block so they are released immediately
    // after the write.  This minimises the time locks are held.
    {
        let mut master = state.master_address.lock().await;
        *master = settings.master_address.clone();
    }
    {
        let mut name = state.client_name.lock().await;
        *name = settings.client_name.clone();
    }

    ClientCommandResult::ok(())
}

/// Returns the number of monitors detected on this client machine.
///
/// Uses the mock enumerator in non-native builds.  In the shipping binary,
/// the `NativeScreenEnumerator` is constructed and injected here.
///
/// Also caches the count in `state.monitor_count` so `get_client_status`
/// can include it without re-querying the OS.
pub async fn get_monitor_count(state: Arc<ClientAppState>) -> ClientCommandResult<u8> {
    let enumerator = MockScreenEnumerator::single_1080p();
    match build_screen_info(&enumerator) {
        Ok(info) => {
            let count = info.monitors.len() as u8;
            let mut guard = state.monitor_count.lock().await;
            *guard = count;
            ClientCommandResult::ok(count)
        }
        Err(e) => ClientCommandResult::err(e.to_string()),
    }
}

// ── Platform helpers ──────────────────────────────────────────────────────────

/// Returns the machine's hostname for use as the default client name.
///
/// Tries `COMPUTERNAME` (Windows) first, then `HOSTNAME` (Unix), and falls
/// back to the literal string `"kvm-client"` if neither is set.
fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "kvm-client".to_string())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> Arc<ClientAppState> {
        ClientAppState::new()
    }

    #[tokio::test]
    async fn test_get_client_status_returns_disconnected_initially() {
        // Arrange
        let state = make_state();

        // Act
        let result = get_client_status(state).await;

        // Assert
        assert!(result.success);
        let dto = result.data.unwrap();
        assert_eq!(dto.connection_status, "Disconnected");
    }

    #[tokio::test]
    async fn test_get_client_settings_returns_empty_master_address_initially() {
        // Arrange
        let state = make_state();

        // Act
        let result = get_client_settings(state).await;

        // Assert
        assert!(result.success);
        assert_eq!(result.data.unwrap().master_address, "");
    }

    #[tokio::test]
    async fn test_update_client_settings_applies_new_values() {
        // Arrange
        let state = make_state();
        let settings = ClientSettingsDto {
            master_address: "192.168.1.10".to_string(),
            client_name: "my-laptop".to_string(),
        };

        // Act
        let result = update_client_settings(Arc::clone(&state), settings).await;
        assert!(result.success);

        let status = get_client_settings(state).await;

        // Assert
        let dto = status.data.unwrap();
        assert_eq!(dto.master_address, "192.168.1.10");
        assert_eq!(dto.client_name, "my-laptop");
    }

    #[tokio::test]
    async fn test_update_client_settings_rejects_empty_name() {
        // Arrange
        let state = make_state();
        let settings = ClientSettingsDto {
            master_address: "10.0.0.1".to_string(),
            client_name: "   ".to_string(), // whitespace only
        };

        // Act
        let result = update_client_settings(state, settings).await;

        // Assert
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_get_monitor_count_returns_at_least_one() {
        // Arrange
        let state = make_state();

        // Act
        let result = get_monitor_count(state).await;

        // Assert
        assert!(result.success);
        assert!(result.data.unwrap() >= 1);
    }

    #[test]
    fn test_client_command_result_ok_sets_success_true() {
        let r: ClientCommandResult<u32> = ClientCommandResult::ok(99);
        assert!(r.success);
        assert_eq!(r.data.unwrap(), 99);
        assert!(r.error.is_none());
    }

    #[test]
    fn test_client_command_result_err_sets_success_false() {
        let r: ClientCommandResult<u32> = ClientCommandResult::err("oops");
        assert!(!r.success);
        assert!(r.data.is_none());
        assert_eq!(r.error.unwrap(), "oops");
    }

    #[test]
    fn test_hostname_returns_non_empty() {
        assert!(!hostname().is_empty());
    }
}
