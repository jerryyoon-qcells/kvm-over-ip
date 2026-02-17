//! # kvm-core
//!
//! Shared library for KVM-Over-IP containing the network protocol codec,
//! domain entities, cryptographic wrappers, and key code translation tables.
//!
//! This crate is used by both the master and client applications.
//! It has zero dependencies on OS APIs, UI frameworks, or network sockets.
//!
//! # Architecture overview (for beginners)
//!
//! KVM-Over-IP is a software KVM switch: it lets you control multiple computers
//! (called "clients") using a single keyboard and mouse connected to one computer
//! (called the "master").  When you move the cursor to the edge of the master
//! screen, control seamlessly switches to the adjacent client.
//!
//! This crate (`kvm-core`) is the shared foundation.  It defines:
//!
//! - **`protocol`** – How bytes travel over the network.  Messages are encoded
//!   into a compact binary format (24-byte header + payload) and decoded back
//!   into typed Rust structs on the other end.
//!
//! - **`domain`** – Pure business logic with no OS dependencies.  The most
//!   important piece is the `VirtualLayout`: a 2-D map of where each screen
//!   lives relative to the master.
//!
//! - **`keymap`** – Translation tables that convert keyboard codes between
//!   platforms (Windows VK codes, Linux X11 KeySyms, macOS CGKeyCodes) and the
//!   canonical representation used on the wire: USB HID Usage IDs.

// Declare the three top-level modules.  Rust will look for each in a
// subdirectory with the same name (e.g., src/protocol/mod.rs).
pub mod domain;
pub mod keymap;
pub mod protocol;

// Re-export the most-used types at the crate root so callers can write
// `kvm_core::VirtualLayout` instead of `kvm_core::domain::layout::VirtualLayout`.
pub use domain::layout::{
    Adjacency, ClientId, ClientScreen, CursorLocation, Edge, EdgeTransition, LayoutError, ScreenId,
    ScreenRegion, VirtualLayout,
};
pub use keymap::hid::HidKeyCode;
pub use protocol::codec::{decode_message, encode_message, ProtocolError};
pub use protocol::messages::KvmMessage;
