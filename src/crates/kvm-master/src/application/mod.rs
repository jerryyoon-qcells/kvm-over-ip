//! Application layer use cases for the master application.
//!
//! # What is the "application" layer? (for beginners)
//!
//! In Clean Architecture the *application* layer sits between the domain
//! (pure business rules) and the infrastructure (OS/network/storage).
//!
//! Use cases in this layer:
//!
//! - **Orchestrate** domain objects to fulfil a user goal (e.g., "route input
//!   to the correct client when the cursor crosses a screen edge").
//! - **Depend on abstractions** (traits) rather than concrete implementations,
//!   so the infrastructure can be swapped without changing this code.
//! - **Contain no OS calls, no network I/O, no file system access**.
//!
//! # Sub-modules
//!
//! - **`route_input`**   – Receives raw input events and decides whether to
//!   process them locally or forward them to a client.  This is the most
//!   critical use case — it runs on every keystroke and mouse movement.
//!
//! - **`manage_clients`** – Maintains the in-memory registry of all known
//!   clients and their connection states.
//!
//! - **`update_layout`** – Validates and applies layout changes (screen
//!   positions and adjacencies) coming from the drag-and-drop UI editor.

pub mod manage_clients;
pub mod route_input;
pub mod update_layout;
