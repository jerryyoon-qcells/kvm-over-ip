//! # kvm-core
//!
//! Shared library for KVM-Over-IP containing the network protocol codec,
//! domain entities, cryptographic wrappers, and key code translation tables.
//!
//! This crate is used by both the master and client applications.
//! It has zero dependencies on OS APIs, UI frameworks, or network sockets.

pub mod domain;
pub mod keymap;
pub mod protocol;

/// Re-export commonly used types at the crate root for convenience.
pub use domain::layout::{
    Adjacency, ClientId, ClientScreen, CursorLocation, Edge, EdgeTransition, LayoutError,
    ScreenId, ScreenRegion, VirtualLayout,
};
pub use keymap::hid::HidKeyCode;
pub use protocol::codec::{decode_message, encode_message, ProtocolError};
pub use protocol::messages::KvmMessage;
