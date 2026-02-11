//! Infrastructure layer for the master application.
//!
//! Contains OS-facing adapters: input capture hooks, network sockets,
//! file-system storage, and the Tauri UI command bridge.
//!
//! **Dependency rule**: this layer may depend on `application` and `kvm_core`,
//! but MUST NOT be imported by the `application` or domain layers.

pub mod input_capture;
pub mod network;
pub mod storage;
pub mod ui_bridge;
