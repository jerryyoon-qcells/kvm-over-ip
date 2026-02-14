//! Platform-specific input emulation implementations.
//!
//! The correct implementation is selected at compile time via `#[cfg(target_os = ...)]`.
//!
//! # How platform selection works (for beginners)
//!
//! The `#[cfg(target_os = "windows")]` attribute is a *conditional compilation*
//! flag.  Rust's compiler only includes code marked with this attribute when
//! the target OS matches.  So when you compile for Windows, only the `windows`
//! module is compiled; the `linux` and `macos` modules are excluded entirely.
//!
//! This is different from runtime OS detection: the platform-specific code is
//! simply not present in the binary for the other platforms.  This is efficient
//! and also prevents "dead code" warnings for OS-specific APIs.
//!
//! | Module    | OS       | API used                                       |
//! |-----------|----------|------------------------------------------------|
//! | `windows` | Windows  | `SendInput` Win32 API                          |
//! | `linux`   | Linux    | X11 XTest extension (`XTestFakeKeyEvent`)      |
//! | `macos`   | macOS    | CoreGraphics (`CGEventCreateKeyboardEvent`)    |
//! | `mock`    | any      | Records events in a `Vec` (for tests)          |

pub mod mock;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;
