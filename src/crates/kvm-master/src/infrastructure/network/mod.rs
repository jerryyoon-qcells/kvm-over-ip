//! Network infrastructure for the master application.
//!
//! # Sub-modules
//!
//! - **`connection_manager`** – Manages the pairing state machine and the per-client
//!   TCP control channel lifecycle.  Handles PIN generation, hash verification,
//!   lockout on repeated failures, and session token issuance.
//!
//! - **`discovery`** – Listens for UDP `AnnounceMessage` broadcasts from clients
//!   on the local network and notifies the application layer via an async channel.
//!   This is how clients are found without manual IP configuration.

pub mod connection_manager;
pub mod discovery;
