//! Domain layer for kvm-web-bridge.
//!
//! The domain layer contains pure business-logic types that have no dependencies
//! on I/O, networking, or external frameworks.  This makes them easy to test in
//! isolation and portable to any runtime or platform.
//!
//! # What belongs in the domain layer?
//!
//! - Message types (the JSON "language" between browser and bridge)
//! - Configuration structures
//! - Session identity types
//! - Error types that describe business-logic failures
//!
//! # What does NOT belong here?
//!
//! - Any `tokio`, `TcpStream`, or `WebSocket` types
//! - File I/O or environment variable reading
//! - Anything that could block or fail due to external state

// Declare the sub-modules that make up the domain layer.
pub mod config;
pub mod messages;

// Re-export the most commonly needed types at the domain module boundary
// so callers can write `domain::BridgeConfig` instead of the longer path.
pub use config::BridgeConfig;
pub use messages::{BrowserToMasterMsg, InputEventJson, MasterToBrowserMsg};
