//! Storage infrastructure: configuration file persistence.
//!
//! This module provides a thin adapter between the application and the
//! file system.  The `config` sub-module handles:
//!
//! - Reading the TOML configuration file from the platform-appropriate directory.
//! - Writing changes back to disk when the user modifies settings.
//! - Providing sensible defaults when the file does not exist yet (first run).
//!
//! Keeping storage concerns here — rather than scattered throughout the
//! application — means we can change the file format (e.g., switch to JSON)
//! without touching any other part of the codebase.

pub mod config;
