//! Infrastructure layer for the client application.
//!
//! Contains OS-facing adapters: input emulation APIs, TCP/UDP network I/O,
//! screen enumeration, and the Tauri UI command bridge.
//!
//! **Dependency rule**: this layer may depend on `application` and `kvm_core`,
//! but MUST NOT be imported by the `application` or domain layers.

pub mod input_emulation;
pub mod network;
pub mod screen_info;
pub mod ui_bridge;
