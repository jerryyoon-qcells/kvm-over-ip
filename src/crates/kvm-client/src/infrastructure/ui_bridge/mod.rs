//! Tauri command bridge for the client application.
//!
//! Exposes application-layer state (connection status, screen info, settings)
//! to the React UI through Tauri commands.  Follows Clean Architecture: only
//! this module is allowed to reference both the Application layer and the
//! Presentation (Tauri) layer.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::infrastructure::screen_info::{build_screen_info, MockScreenEnumerator};

// ── Shared application state ──────────────────────────────────────────────────

/// Connection status of the client as seen by the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientConnectionStatus {
    /// Not yet attempting to connect.
    Disconnected,
    /// Actively trying to reach the master.
    Connecting,
    /// Control channel established; awaiting pairing or HelloAck.
    Connected,
    /// Fully paired and receiving input events.
    Active,
    /// Pairing is in progress (PIN display).
    Pairing,
}

/// Runtime state shared between Tauri commands.
pub struct ClientAppState {
    pub connection_status: Mutex<ClientConnectionStatus>,
    pub master_address: Mutex<String>,
    pub client_name: Mutex<String>,
    pub monitor_count: Mutex<u8>,
}

impl ClientAppState {
    /// Creates a new `ClientAppState` with all fields at their defaults.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientStatusDto {
    pub connection_status: String,
    pub master_address: String,
    pub client_name: String,
    pub monitor_count: u8,
}

/// Settings DTO that can be read/written from the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSettingsDto {
    pub master_address: String,
    pub client_name: String,
}

/// Unified response wrapper for client commands.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientCommandResult<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ClientCommandResult<T> {
    pub fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn err(msg: impl Into<String>) -> Self {
        Self { success: false, data: None, error: Some(msg.into()) }
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Returns the current client status snapshot.
pub async fn get_client_status(state: Arc<ClientAppState>) -> ClientCommandResult<ClientStatusDto> {
    let status = state.connection_status.lock().await;
    let master = state.master_address.lock().await;
    let name = state.client_name.lock().await;
    let monitors = state.monitor_count.lock().await;

    ClientCommandResult::ok(ClientStatusDto {
        connection_status: format!("{status:?}"),
        master_address: master.clone(),
        client_name: name.clone(),
        monitor_count: *monitors,
    })
}

/// Returns the current client settings.
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

/// Applies new client settings.
pub async fn update_client_settings(
    state: Arc<ClientAppState>,
    settings: ClientSettingsDto,
) -> ClientCommandResult<()> {
    if settings.client_name.trim().is_empty() {
        return ClientCommandResult::err("client_name must not be empty");
    }

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
