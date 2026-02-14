//! Infrastructure layer for the client application.
//!
//! Contains OS-facing adapters: input emulation APIs, TCP/UDP network I/O,
//! screen enumeration, and the Tauri UI command bridge.
//!
//! **Dependency rule**: this layer may depend on `application` and `kvm_core`,
//! but MUST NOT be imported by the `application` or domain layers.
//!
//! # Sub-modules
//!
//! - **`input_emulation`** – OS-specific implementations of `PlatformInputEmulator`.
//!   The correct implementation is selected at compile time using `#[cfg(target_os)]`.
//!   A `MockInputEmulator` is also provided for tests.
//!
//! - **`network`** – TCP client that connects to the master, handles the protocol
//!   handshake, reads framed messages from the socket, and reconnects automatically
//!   if the connection drops.
//!
//! - **`screen_info`** – OS-specific monitor enumeration.  On Windows it calls
//!   `EnumDisplayMonitors`; on Linux it queries Xlib; on macOS it uses `CGDisplay`.
//!
//! - **`ui_bridge`** – Tauri command handlers that expose client state (connection
//!   status, settings) to the React UI.

pub mod input_emulation;
pub mod network;
pub mod screen_info;
pub mod ui_bridge;
