//! TOML-based configuration persistence for the master application.
//!
//! Reads and writes `AppConfig` to the platform-appropriate config file:
//! - Windows:  `%APPDATA%\KVMOverIP\config.toml`
//! - Linux:    `~/.config/kvmoverip/config.toml`
//! - macOS:    `~/Library/Application Support/KVMOverIP/config.toml`
//!
//! # What is TOML? (for beginners)
//!
//! TOML (Tom's Obvious Minimal Language) is a configuration file format designed
//! to be easy to read and write.  It looks similar to INI files but with more
//! data types.  Example:
//!
//! ```toml
//! [network]
//! control_port = 24800
//! bind_address = "0.0.0.0"
//!
//! [master]
//! disable_hotkey = "ScrollLock+ScrollLock"
//! autostart = true
//! ```
//!
//! The `serde` library provides automatic serialisation/deserialisation between
//! Rust structs and TOML text.  The `#[derive(Serialize, Deserialize)]` macros
//! generate all the boilerplate code at compile time.
//!
//! # Serde default values
//!
//! Fields annotated with `#[serde(default = "some_fn")]` use the return value
//! of `some_fn()` when the field is absent from the TOML file.  This allows
//! the app to work correctly on first run (before a config file exists) and
//! when upgrading from an older config file that is missing newer fields.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Error type for configuration file operations.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The platform config directory could not be determined.
    #[error("could not determine platform config directory")]
    NoPlatformConfigDir,

    /// A file system I/O error occurred.
    #[error("I/O error accessing config at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The TOML content could not be parsed.
    #[error("failed to parse config TOML: {0}")]
    Parse(#[from] toml::de::Error),

    /// The config could not be serialized to TOML.
    #[error("failed to serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),
}

// ── Config schema types ───────────────────────────────────────────────────────

/// Top-level application configuration stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub master: MasterConfig,
    pub network: NetworkConfig,
    pub layout: LayoutConfig,
    #[serde(default)]
    pub clients: Vec<ClientEntry>,
}

/// General master behaviour settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MasterConfig {
    /// Schema version string – bump when breaking changes are introduced.
    #[serde(default = "default_version")]
    pub version: String,
    /// Human-readable hotkey description (e.g. `"ScrollLock+ScrollLock"`).
    #[serde(default = "default_hotkey")]
    pub disable_hotkey: String,
    /// Whether the master starts minimised to tray on OS login.
    #[serde(default = "default_true")]
    pub autostart: bool,
    /// `tracing` log level: `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"`.
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

/// Network port and bind-address settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkConfig {
    /// TCP port for the TLS control channel.
    #[serde(default = "default_control_port")]
    pub control_port: u16,
    /// UDP port for the DTLS input channel.
    #[serde(default = "default_input_port")]
    pub input_port: u16,
    /// UDP port for LAN device discovery broadcasts.
    #[serde(default = "default_discovery_port")]
    pub discovery_port: u16,
    /// IP address to bind all sockets to.  `"0.0.0.0"` binds all interfaces.
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
}

/// Virtual layout of the master screen plus positioned client screens.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutConfig {
    /// Physical width of the master monitor in pixels.
    #[serde(default = "default_screen_width")]
    pub master_screen_width: u32,
    /// Physical height of the master monitor in pixels.
    #[serde(default = "default_screen_height")]
    pub master_screen_height: u32,
    /// Positioned client screens.
    #[serde(default)]
    pub clients: Vec<ClientLayoutEntry>,
}

/// Positioned layout entry for a single client screen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientLayoutEntry {
    /// UUID identifying the client.
    pub client_id: Uuid,
    /// Display name shown in the UI.
    pub name: String,
    /// Horizontal offset in pixels relative to the master's top-left corner.
    pub x_offset: i32,
    /// Vertical offset in pixels relative to the master's top-left corner.
    pub y_offset: i32,
    /// Client screen width in pixels.
    pub width: u32,
    /// Client screen height in pixels.
    pub height: u32,
}

/// Persisted record of a known/paired client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientEntry {
    /// UUID identifying the client.
    pub client_id: Uuid,
    /// Display name for the client.
    pub name: String,
    /// Optional static IP – if absent, discovery resolves the address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// SHA-256 derived pairing hash stored after successful PIN exchange.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairing_hash: Option<String>,
}

// ── Default helpers ───────────────────────────────────────────────────────────

fn default_version() -> String {
    "1.0".to_string()
}
fn default_hotkey() -> String {
    "ScrollLock+ScrollLock".to_string()
}
fn default_true() -> bool {
    true
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_control_port() -> u16 {
    24800
}
fn default_input_port() -> u16 {
    24801
}
fn default_discovery_port() -> u16 {
    24802
}
fn default_bind_address() -> String {
    "0.0.0.0".to_string()
}
fn default_screen_width() -> u32 {
    1920
}
fn default_screen_height() -> u32 {
    1080
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            master: MasterConfig::default(),
            network: NetworkConfig::default(),
            layout: LayoutConfig::default(),
            clients: Vec::new(),
        }
    }
}

impl Default for MasterConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            disable_hotkey: default_hotkey(),
            autostart: default_true(),
            log_level: default_log_level(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            control_port: default_control_port(),
            input_port: default_input_port(),
            discovery_port: default_discovery_port(),
            bind_address: default_bind_address(),
        }
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            master_screen_width: default_screen_width(),
            master_screen_height: default_screen_height(),
            clients: Vec::new(),
        }
    }
}

// ── Config repository ─────────────────────────────────────────────────────────

/// Determines the platform-appropriate directory for the config file.
///
/// # Errors
///
/// Returns [`ConfigError::NoPlatformConfigDir`] when the platform config base
/// directory cannot be determined from the environment.
pub fn config_dir() -> Result<PathBuf, ConfigError> {
    platform_config_dir().ok_or(ConfigError::NoPlatformConfigDir)
}

/// Resolves the full path to the config file.
///
/// # Errors
///
/// Returns [`ConfigError::NoPlatformConfigDir`] if the base directory cannot be
/// determined.
pub fn config_file_path() -> Result<PathBuf, ConfigError> {
    Ok(config_dir()?.join("config.toml"))
}

/// Loads `AppConfig` from disk, returning `AppConfig::default()` if the file
/// does not yet exist.
///
/// # Errors
///
/// Returns [`ConfigError::Io`] for file-system errors other than "not found",
/// and [`ConfigError::Parse`] if the TOML is malformed.
pub fn load_config() -> Result<AppConfig, ConfigError> {
    let path = config_file_path()?;

    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let cfg: AppConfig = toml::from_str(&content)?;
            Ok(cfg)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(e) => Err(ConfigError::Io { path, source: e }),
    }
}

/// Persists `config` to disk.
///
/// Creates the config directory and file if they do not exist.
///
/// # Errors
///
/// Returns [`ConfigError::Io`] for file-system failures or
/// [`ConfigError::Serialize`] if serialization fails.
pub fn save_config(config: &AppConfig) -> Result<(), ConfigError> {
    let path = config_file_path()?;

    // Ensure directory exists before writing.
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|source| ConfigError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
    }

    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content).map_err(|source| ConfigError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(())
}

/// Resolves the platform config base directory without the `KVMOverIP` subdirectory.
fn platform_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // %APPDATA% e.g. C:\Users\<user>\AppData\Roaming
        std::env::var_os("APPDATA")
            .map(|p| PathBuf::from(p).join("KVMOverIP"))
    }

    #[cfg(target_os = "linux")]
    {
        // XDG_CONFIG_HOME or ~/.config
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("kvmoverip"))
    }

    #[cfg(target_os = "macos")]
    {
        // ~/Library/Application Support/KVMOverIP
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library").join("Application Support").join("KVMOverIP"))
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        // Fallback for unsupported platforms.
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── AppConfig defaults ────────────────────────────────────────────────────

    #[test]
    fn test_app_config_default_has_expected_ports() {
        // Arrange / Act
        let cfg = AppConfig::default();

        // Assert
        assert_eq!(cfg.network.control_port, 24800);
        assert_eq!(cfg.network.input_port, 24801);
        assert_eq!(cfg.network.discovery_port, 24802);
    }

    #[test]
    fn test_app_config_default_has_expected_screen_dimensions() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.layout.master_screen_width, 1920);
        assert_eq!(cfg.layout.master_screen_height, 1080);
    }

    #[test]
    fn test_app_config_default_has_no_clients() {
        let cfg = AppConfig::default();
        assert!(cfg.clients.is_empty());
        assert!(cfg.layout.clients.is_empty());
    }

    #[test]
    fn test_master_config_default_log_level_is_info() {
        let cfg = MasterConfig::default();
        assert_eq!(cfg.log_level, "info");
    }

    #[test]
    fn test_master_config_default_autostart_is_true() {
        let cfg = MasterConfig::default();
        assert!(cfg.autostart);
    }

    // ── TOML round-trip ───────────────────────────────────────────────────────

    #[test]
    fn test_app_config_serializes_and_deserializes_round_trip() {
        // Arrange
        let mut cfg = AppConfig::default();
        cfg.network.control_port = 9000;
        cfg.layout.master_screen_width = 2560;

        // Act
        let toml_str = toml::to_string_pretty(&cfg).expect("serialize");
        let restored: AppConfig = toml::from_str(&toml_str).expect("deserialize");

        // Assert
        assert_eq!(cfg, restored);
    }

    #[test]
    fn test_app_config_with_client_entries_round_trips() {
        // Arrange
        let client_id = Uuid::new_v4();
        let mut cfg = AppConfig::default();
        cfg.clients.push(ClientEntry {
            client_id,
            name: "dev-linux".to_string(),
            host: Some("192.168.1.100".to_string()),
            pairing_hash: Some("sha256:abc123".to_string()),
        });
        cfg.layout.clients.push(ClientLayoutEntry {
            client_id,
            name: "dev-linux".to_string(),
            x_offset: 1920,
            y_offset: 0,
            width: 2560,
            height: 1440,
        });

        // Act
        let toml_str = toml::to_string_pretty(&cfg).expect("serialize");
        let restored: AppConfig = toml::from_str(&toml_str).expect("deserialize");

        // Assert
        assert_eq!(cfg, restored);
        assert_eq!(restored.clients[0].client_id, client_id);
        assert_eq!(restored.clients[0].name, "dev-linux");
    }

    #[test]
    fn test_client_entry_without_optional_fields_round_trips() {
        // Arrange: host and pairing_hash are None → should be omitted from TOML
        let entry = ClientEntry {
            client_id: Uuid::new_v4(),
            name: "bare-client".to_string(),
            host: None,
            pairing_hash: None,
        };
        let mut cfg = AppConfig::default();
        cfg.clients.push(entry.clone());

        // Act
        let toml_str = toml::to_string_pretty(&cfg).expect("serialize");

        // Assert – the optional fields must not appear in the TOML output
        assert!(!toml_str.contains("host"), "None host must be omitted");
        assert!(
            !toml_str.contains("pairing_hash"),
            "None pairing_hash must be omitted"
        );

        let restored: AppConfig = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(restored.clients[0].host, None);
        assert_eq!(restored.clients[0].pairing_hash, None);
    }

    #[test]
    fn test_deserialize_minimal_toml_uses_defaults() {
        // Arrange: minimal TOML with only required sections
        let toml_str = r#"
[master]
[network]
[layout]
"#;

        // Act
        let cfg: AppConfig = toml::from_str(toml_str).expect("deserialize minimal");

        // Assert
        assert_eq!(cfg.network.control_port, 24800);
        assert_eq!(cfg.master.log_level, "info");
        assert!(cfg.clients.is_empty());
    }

    #[test]
    fn test_deserialize_partial_network_overrides_defaults() {
        // Arrange
        let toml_str = r#"
[master]
[network]
control_port = 9999
[layout]
"#;

        // Act
        let cfg: AppConfig = toml::from_str(toml_str).expect("deserialize partial");

        // Assert
        assert_eq!(cfg.network.control_port, 9999);
        // Unspecified fields keep their defaults
        assert_eq!(cfg.network.input_port, 24801);
    }

    #[test]
    fn test_deserialize_invalid_toml_returns_parse_error() {
        // Arrange
        let bad_toml = "[[[ not valid toml";

        // Act
        let result: Result<AppConfig, toml::de::Error> = toml::from_str(bad_toml);

        // Assert
        assert!(result.is_err());
    }

    // ── load_config from temp directory ──────────────────────────────────────

    #[test]
    fn test_load_config_returns_default_when_file_absent() {
        // Arrange: use a known non-existent path to exercise the NotFound path
        let path = PathBuf::from("/nonexistent/path/that/cannot/exist/config.toml");
        let content = std::fs::read_to_string(&path);

        // Act
        let result = match content {
            Ok(s) => toml::from_str::<AppConfig>(&s).map_err(|e| format!("parse: {e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
            Err(e) => Err(format!("io: {e}")),
        };

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), AppConfig::default());
    }

    #[test]
    fn test_save_and_load_config_round_trip_via_temp_dir() {
        // Arrange
        let dir = std::env::temp_dir().join(format!("kvm_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");

        let mut cfg = AppConfig::default();
        cfg.network.control_port = 12345;
        cfg.master.log_level = "debug".to_string();

        // Act – serialize and write manually (mirrors save_config logic)
        let content = toml::to_string_pretty(&cfg).unwrap();
        std::fs::write(&path, &content).unwrap();
        let loaded: AppConfig = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        // Assert
        assert_eq!(loaded.network.control_port, 12345);
        assert_eq!(loaded.master.log_level, "debug");

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    // ── config_dir path formation ─────────────────────────────────────────────

    #[test]
    fn test_platform_config_dir_returns_some_on_this_platform() {
        // This test verifies the function returns Some on the current platform.
        // It may fail if the environment variable is unset in a stripped container.
        let result = platform_config_dir();
        // We only assert it is Some when the relevant env var is available.
        #[cfg(target_os = "windows")]
        if std::env::var_os("APPDATA").is_some() {
            assert!(result.is_some());
        }
        #[cfg(target_os = "linux")]
        {
            let has_xdg = std::env::var_os("XDG_CONFIG_HOME").is_some();
            let has_home = std::env::var_os("HOME").is_some();
            if has_xdg || has_home {
                assert!(result.is_some());
            }
        }
        #[cfg(target_os = "macos")]
        if std::env::var_os("HOME").is_some() {
            assert!(result.is_some());
        }
    }

    #[test]
    fn test_config_file_path_ends_with_config_toml() {
        let path_result = config_file_path();
        if let Ok(path) = path_result {
            assert!(
                path.ends_with("config.toml"),
                "config file must be named config.toml, got {path:?}"
            );
        }
        // If NoPlatformConfigDir is returned (e.g. in a stripped CI env) that is also acceptable.
    }
}
