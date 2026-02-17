//! Infrastructure layer for kvm-web-bridge.
//!
//! The infrastructure layer handles all I/O: accepting WebSocket connections
//! from browsers and opening TCP connections to the KVM master.
//!
//! # Responsibilities
//!
//! - Binding a TCP listener for browser WebSocket connections
//! - Performing the WebSocket HTTP upgrade handshake
//! - Opening and managing TCP connections to the KVM master
//! - Reading and writing binary KVM messages over TCP
//! - Spawning per-session Tokio tasks
//! - Handling the graceful shutdown signal
//!
//! # What does NOT belong here?
//!
//! - Protocol translation logic (that is the application layer)
//! - Message type definitions (that is the domain layer)
//! - Configuration parsing (that is done in `main.rs`)

pub mod master_conn;
pub mod ws_server;

// Re-export the primary entry points so `main.rs` can call them concisely.
pub use ws_server::run_server;
