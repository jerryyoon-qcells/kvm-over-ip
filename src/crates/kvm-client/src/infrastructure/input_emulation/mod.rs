//! Platform-specific input emulation implementations.
//!
//! The correct implementation is selected at compile time via `#[cfg(target_os = ...)]`.

pub mod mock;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;
