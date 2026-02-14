//! Infrastructure layer for the master application.
//!
//! Contains OS-facing adapters: input capture hooks, network sockets,
//! file-system storage, and the Tauri UI command bridge.
//!
//! **Dependency rule**: this layer may depend on `application` and `kvm_core`,
//! but MUST NOT be imported by the `application` or domain layers.
//!
//! # What is "infrastructure"? (for beginners)
//!
//! In Clean Architecture the *infrastructure* layer is the outermost ring.  It
//! is the only place where the code is allowed to:
//!
//! - Call OS APIs (Windows hooks, `SendInput`, file system).
//! - Open or listen on network sockets.
//! - Access hardware.
//! - Interact with external services.
//!
//! All infrastructure modules implement traits defined in the *application*
//! layer (e.g., `InputSource`, `InputTransmitter`) so the rest of the system
//! does not need to know which concrete implementation is used.
//!
//! # Sub-modules
//!
//! - **`input_capture`** – Windows low-level hooks that intercept keyboard and
//!   mouse events before they reach the local desktop.
//! - **`network`**       – TCP control channel, pairing state machine, and UDP
//!   discovery responder.
//! - **`storage`**       – TOML configuration file read/write.
//! - **`ui_bridge`**     – Tauri command handlers that expose application state
//!   to the React UI.

pub mod input_capture;
pub mod network;
pub mod storage;
pub mod ui_bridge;
