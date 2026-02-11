//! kvm-master library entry point.
//!
//! Re-exports all public modules so that integration tests in `tests/`
//! and the binary entry point in `main.rs` share the same module tree.

pub mod application;
pub mod infrastructure;
