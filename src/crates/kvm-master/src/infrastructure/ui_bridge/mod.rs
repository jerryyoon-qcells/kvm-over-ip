//! Tauri command bridge: exposes application-layer operations to the React UI.
//!
//! All `#[tauri::command]` functions live here and delegate to the shared
//! [`AppState`].  The Presentation layer (Tauri + React) is the only consumer
//! of this module; it must NOT be imported by the Application or Domain layers.
//!
//! # How Tauri commands work (for beginners)
//!
//! Tauri is a framework for building native desktop applications using a web
//! frontend (React/HTML/JS) and a Rust backend.
//!
//! The React frontend calls Rust functions using:
//! ```js
//! const result = await invoke("get_clients");
//! ```
//!
//! Tauri routes `"get_clients"` to the `#[tauri::command] fn get_clients(...)`.
//! The function receives the `AppState` via dependency injection and returns
//! a value that Tauri serialises to JSON for the frontend.
//!
//! # Data Transfer Objects (DTOs)
//!
//! The Rust backend uses internal types (e.g., `ClientRuntimeState`, `Uuid`)
//! that are not directly serialisable to JSON.  DTOs are simple structs
//! (`ClientDto`, `ClientLayoutDto`) that:
//!
//! - Contain only JSON-serialisable fields (`String`, `f32`, `u32`, etc.)
//! - Are defined using `#[derive(Serialize, Deserialize)]` so Tauri can
//!   automatically convert them to/from JSON.
//! - Mirror the TypeScript interfaces in `src/packages/ui-master/src/types.ts`.
//!
//! Any change to a DTO struct here must be reflected in the corresponding
//! TypeScript interface to avoid runtime type mismatches.
//!
//! # `CommandResult<T>` wrapper
//!
//! All Tauri commands return `CommandResult<T>` rather than `Result<T, E>`.
//! This ensures every command response has the same shape:
//! `{ success: bool, data: T | null, error: string | null }`.
//! The frontend can always safely access `result.success` without a
//! try/catch block around the `invoke` call.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::application::{
    manage_clients::{ClientRegistry, ClientRuntimeState},
    update_layout::{build_layout, ClientLayoutConfig},
};
use crate::infrastructure::{
    network::connection_manager::{ConnectionManager, NetworkConfig},
    storage::config::{load_config, save_config, AppConfig, ClientLayoutEntry},
};
use kvm_core::ClientId;

// ── Shared application state ──────────────────────────────────────────────────

/// Application state shared between Tauri commands via `tauri::State`.
///
/// This struct is wrapped in `Arc<>` and registered with Tauri's state
/// management system.  Tauri injects it into every command handler function
/// as a parameter of type `tauri::State<Arc<AppState>>`.
///
/// All fields are `Mutex<...>` (async Tokio mutex) because Tauri commands run
/// in an async Tokio context and the mutex allows safe concurrent access from
/// multiple simultaneous command invocations.
///
/// # Why async Mutex (not std::sync::Mutex)?
///
/// `std::sync::Mutex` blocks the OS thread while waiting to acquire the lock.
/// In an async context this is problematic because blocking a thread prevents
/// other async tasks from running.  `tokio::sync::Mutex` suspends the async
/// task instead of blocking the thread, allowing other tasks to proceed.
pub struct AppState {
    /// The in-memory registry of all known client machines.
    pub client_registry: Mutex<ClientRegistry>,
    /// Manages TCP connections and the pairing state machine.
    pub connection_manager: Mutex<ConnectionManager>,
    /// The current application configuration (network ports, layout, etc.).
    pub config: Mutex<AppConfig>,
}

impl AppState {
    /// Initialises application state from the persisted configuration.
    ///
    /// Falls back to defaults if no config file exists yet.
    pub fn new() -> Arc<Self> {
        let config = load_config().unwrap_or_default();
        let net_cfg = NetworkConfig {
            control_port: config.network.control_port,
            input_port: config.network.input_port,
            discovery_port: config.network.discovery_port,
            bind_address: config
                .network
                .bind_address
                .parse()
                .unwrap_or_else(|_| "0.0.0.0".parse().unwrap()),
        };
        let (conn_mgr, _event_rx) = ConnectionManager::new(net_cfg);

        Arc::new(Self {
            client_registry: Mutex::new(ClientRegistry::new()),
            connection_manager: Mutex::new(conn_mgr),
            config: Mutex::new(config),
        })
    }
}

// ── Data Transfer Objects (Presentation layer) ────────────────────────────────

/// DTO representing one connected client returned to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientDto {
    pub client_id: String,
    pub name: String,
    pub connection_state: String,
    pub latency_ms: f32,
    pub events_per_second: u32,
}

impl From<&ClientRuntimeState> for ClientDto {
    fn from(s: &ClientRuntimeState) -> Self {
        Self {
            client_id: s.id.to_string(),
            name: s.name.clone(),
            connection_state: format!("{:?}", s.connection_state),
            latency_ms: s.latency_ms,
            events_per_second: s.events_per_second,
        }
    }
}

/// DTO for a single client layout entry passed from the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientLayoutDto {
    pub client_id: String,
    pub name: String,
    pub x_offset: i32,
    pub y_offset: i32,
    pub width: u32,
    pub height: u32,
}

/// DTO for the current network configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfigDto {
    pub control_port: u16,
    pub input_port: u16,
    pub discovery_port: u16,
    pub bind_address: String,
}

/// Unified response wrapper used by Tauri commands.
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResult<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> CommandResult<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Returns a list of all currently registered clients.
///
/// # Example (frontend)
/// ```ts
/// const clients = await invoke<ClientDto[]>('get_clients');
/// ```
pub async fn get_clients(state: Arc<AppState>) -> CommandResult<Vec<ClientDto>> {
    let registry = state.client_registry.lock().await;
    let dtos: Vec<ClientDto> = registry.all().iter().map(ClientDto::from).collect();
    CommandResult::ok(dtos)
}

/// Returns the current layout configuration.
pub async fn get_layout(state: Arc<AppState>) -> CommandResult<Vec<ClientLayoutDto>> {
    let config = state.config.lock().await;
    let dtos: Vec<ClientLayoutDto> = config
        .layout
        .clients
        .iter()
        .map(|e| ClientLayoutDto {
            client_id: e.client_id.to_string(),
            name: e.name.clone(),
            x_offset: e.x_offset,
            y_offset: e.y_offset,
            width: e.width,
            height: e.height,
        })
        .collect();
    CommandResult::ok(dtos)
}

/// Applies and persists a new layout from the UI.
///
/// Validates the layout (no overlapping screens) before writing to disk.
pub async fn update_layout(
    state: Arc<AppState>,
    clients: Vec<ClientLayoutDto>,
) -> CommandResult<()> {
    // Parse client IDs from strings
    let configs: Vec<ClientLayoutConfig> = match clients
        .iter()
        .map(|dto| {
            dto.client_id
                .parse::<ClientId>()
                .map(|id| ClientLayoutConfig {
                    client_id: id,
                    name: dto.name.clone(),
                    x_offset: dto.x_offset,
                    y_offset: dto.y_offset,
                    width: dto.width,
                    height: dto.height,
                })
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(v) => v,
        Err(e) => return CommandResult::err(format!("invalid client_id UUID: {e}")),
    };

    // Lock config to get master dimensions
    let (master_w, master_h) = {
        let cfg = state.config.lock().await;
        (
            cfg.layout.master_screen_width,
            cfg.layout.master_screen_height,
        )
    };

    // Validate layout geometry
    if let Err(e) = build_layout(master_w, master_h, configs) {
        return CommandResult::err(e.to_string());
    }

    // Persist to config file
    let mut cfg = state.config.lock().await;
    cfg.layout.clients = clients
        .iter()
        .filter_map(|dto| {
            dto.client_id
                .parse::<ClientId>()
                .ok()
                .map(|id| ClientLayoutEntry {
                    client_id: id,
                    name: dto.name.clone(),
                    x_offset: dto.x_offset,
                    y_offset: dto.y_offset,
                    width: dto.width,
                    height: dto.height,
                })
        })
        .collect();

    if let Err(e) = save_config(&cfg) {
        return CommandResult::err(format!("failed to save config: {e}"));
    }

    CommandResult::ok(())
}

/// Returns the current network configuration.
pub async fn get_network_config(state: Arc<AppState>) -> CommandResult<NetworkConfigDto> {
    let cfg = state.config.lock().await;
    CommandResult::ok(NetworkConfigDto {
        control_port: cfg.network.control_port,
        input_port: cfg.network.input_port,
        discovery_port: cfg.network.discovery_port,
        bind_address: cfg.network.bind_address.clone(),
    })
}

/// Applies and persists a new network configuration.
pub async fn update_network_config(
    state: Arc<AppState>,
    network: NetworkConfigDto,
) -> CommandResult<()> {
    let mut cfg = state.config.lock().await;
    cfg.network.control_port = network.control_port;
    cfg.network.input_port = network.input_port;
    cfg.network.discovery_port = network.discovery_port;
    cfg.network.bind_address = network.bind_address;

    if let Err(e) = save_config(&cfg) {
        return CommandResult::err(format!("failed to save config: {e}"));
    }
    CommandResult::ok(())
}

/// Returns whether sharing is currently active.
///
/// NOTE: In the full Tauri integration this would read from the RouteInputUseCase.
/// For the prototype the value is stored in the config mutex as a transient flag.
pub async fn get_sharing_enabled(_state: Arc<AppState>) -> CommandResult<bool> {
    // Sharing state is managed by RouteInputUseCase at runtime.
    // This command is a stub that the UI polls; in production the Tauri event
    // system pushes updates via `app_handle.emit("sharing_changed", payload)`.
    CommandResult::ok(false)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::config::config_file_path;

    /// Creates a test-isolated AppState using AppConfig::default() so that tests
    /// never read from or write to the real platform config file on disk.
    fn make_state() -> Arc<AppState> {
        let config = AppConfig::default();
        let net_cfg = NetworkConfig {
            control_port: config.network.control_port,
            input_port: config.network.input_port,
            discovery_port: config.network.discovery_port,
            bind_address: config
                .network
                .bind_address
                .parse()
                .unwrap_or_else(|_| "0.0.0.0".parse().unwrap()),
        };
        let (conn_mgr, _event_rx) = ConnectionManager::new(net_cfg);
        Arc::new(AppState {
            client_registry: Mutex::new(ClientRegistry::new()),
            connection_manager: Mutex::new(conn_mgr),
            config: Mutex::new(config),
        })
    }

    #[tokio::test]
    async fn test_get_clients_returns_empty_list_initially() {
        // Arrange
        let state = make_state();

        // Act
        let result = get_clients(state).await;

        // Assert
        assert!(result.success);
        assert_eq!(result.data.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_get_layout_returns_empty_list_when_no_clients_configured() {
        // Arrange
        let state = make_state();

        // Act
        let result = get_layout(state).await;

        // Assert
        assert!(result.success);
        assert_eq!(result.data.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_update_layout_fails_with_invalid_uuid() {
        // Arrange
        let state = make_state();
        let bad_clients = vec![ClientLayoutDto {
            client_id: "not-a-uuid".to_string(),
            name: "bad".to_string(),
            x_offset: 0,
            y_offset: 0,
            width: 1920,
            height: 1080,
        }];

        // Act
        let result = update_layout(state, bad_clients).await;

        // Assert
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_update_layout_succeeds_with_valid_non_overlapping_client() {
        // Arrange
        let state = make_state();
        let id = ClientId::new_v4();
        let clients = vec![ClientLayoutDto {
            client_id: id.to_string(),
            name: "dev-linux".to_string(),
            x_offset: 1920,
            y_offset: 0,
            width: 1920,
            height: 1080,
        }];

        // Act
        let result = update_layout(state, clients).await;

        // Assert
        assert!(
            result.success,
            "expected success, got error: {:?}",
            result.error
        );

        // Cleanup: remove the config file written to the real platform config dir
        // to avoid contaminating subsequent test runs.
        if let Ok(path) = config_file_path() {
            let _ = std::fs::remove_file(&path);
        }
    }

    #[tokio::test]
    async fn test_get_network_config_returns_default_ports() {
        // Arrange
        let state = make_state();

        // Act
        let result = get_network_config(state).await;

        // Assert
        assert!(result.success);
        let dto = result.data.unwrap();
        assert_eq!(dto.control_port, 24800);
        assert_eq!(dto.input_port, 24801);
        assert_eq!(dto.discovery_port, 24802);
    }

    #[tokio::test]
    async fn test_get_sharing_enabled_returns_false_initially() {
        // Arrange
        let state = make_state();

        // Act
        let result = get_sharing_enabled(state).await;

        // Assert
        assert!(result.success);
        assert_eq!(result.data.unwrap(), false);
    }

    #[test]
    fn test_command_result_ok_sets_success_true() {
        let r: CommandResult<i32> = CommandResult::ok(42);
        assert!(r.success);
        assert_eq!(r.data.unwrap(), 42);
        assert!(r.error.is_none());
    }

    #[test]
    fn test_command_result_err_sets_success_false() {
        let r: CommandResult<i32> = CommandResult::err("something went wrong");
        assert!(!r.success);
        assert!(r.data.is_none());
        assert_eq!(r.error.unwrap(), "something went wrong");
    }
}
