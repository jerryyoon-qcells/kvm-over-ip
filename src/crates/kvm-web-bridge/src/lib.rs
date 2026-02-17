//! kvm-web-bridge library crate.
//!
//! This crate provides a WebSocket-to-native-protocol bridge that allows web
//! browsers to act as KVM-Over-IP clients.
//!
//! # Architecture (clean architecture)
//!
//! ```text
//! Browser (JSON over WebSocket)
//!         ↕
//! [kvm-web-bridge]
//!   ├── domain/           Pure types: JSON message enums, BridgeConfig
//!   ├── application/      Translation: JSON ↔ binary KVM protocol
//!   └── infrastructure/
//!         ├── ws_server/  WebSocket accept loop (tokio-tungstenite)
//!         └── master_conn/ TCP connection to KVM master (kvm-core codec)
//! ```
//!
//! # Layer rules
//!
//! - `domain` has no external dependencies (no I/O, no async, no frameworks).
//! - `application` depends on `domain` and `kvm-core` only.
//! - `infrastructure` depends on all other layers plus `tokio` and `tungstenite`.
//!
//! # For beginners: why this structure?
//!
//! Clean architecture separates *what the program does* (domain + application)
//! from *how it does it* (infrastructure).  This makes the business logic easy
//! to test without a real network, and easy to swap out the transport layer
//! (e.g., to support native WebRTC in the future) without touching the
//! translation logic.

/// Domain layer: pure business-logic types (no I/O).
pub mod domain;

/// Application layer: message translation logic.
pub mod application;

/// Infrastructure layer: WebSocket server and master TCP connection.
pub mod infrastructure;
