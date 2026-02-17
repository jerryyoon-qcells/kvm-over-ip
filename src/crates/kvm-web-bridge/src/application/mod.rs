//! Application layer for kvm-web-bridge.
//!
//! The application layer orchestrates the business logic: it knows *what* to
//! do, but delegates *how* to do it to the infrastructure layer.
//!
//! # Responsibilities
//!
//! - Translating browser JSON messages into binary KVM protocol messages
//! - Translating binary KVM protocol messages into browser JSON messages
//! - Defining the `BridgeError` type for application-level failures
//!
//! # What does NOT belong here?
//!
//! - Opening sockets or listening for connections (that is infrastructure)
//! - Tokio task spawning (that happens in the infrastructure layer)
//! - WebSocket framing (handled by tokio-tungstenite)

pub mod bridge_service;

// Re-export so callers can write `application::bridge_service::translate_browser_to_kvm`
// or more concisely `application::translate_browser_to_kvm`.
pub use bridge_service::{base64_encode, translate_browser_to_kvm, translate_kvm_to_browser, BridgeError};
