//! kvm-master library entry point.
//!
//! Re-exports all public modules so that integration tests in `tests/`
//! and the binary entry point in `main.rs` share the same module tree.
//!
//! # Layer overview (for beginners)
//!
//! The master application follows Clean Architecture:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │ Infrastructure (input capture, network, storage, UI bridge)  │
//! │   └── depends on ↓                                           │
//! │ Application (route_input, manage_clients, update_layout)     │
//! │   └── depends on ↓                                           │
//! │ Domain (VirtualLayout, Adjacency, etc. — from kvm-core)      │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! Dependencies only point inward: domain knows nothing about infrastructure;
//! infrastructure knows everything about domain and application.

/// Application layer: use cases that implement business rules.
pub mod application;

/// Infrastructure layer: OS adapters, network, storage, and UI bridge.
pub mod infrastructure;
