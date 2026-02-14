//! kvm-client library entry point.
//!
//! Re-exports all public modules so that integration tests in `tests/`
//! and the binary entry point in `main.rs` share the same module tree.
//!
//! # What does kvm-client do? (for beginners)
//!
//! The *client* is the remote computer whose keyboard and mouse are being
//! *controlled* by the master.  When the master user moves their cursor
//! to the edge of the master screen and it crosses to the client's virtual
//! position, the master starts forwarding all keyboard and mouse events to
//! the client over TCP.
//!
//! The client application:
//!
//! 1. Connects to the master over TCP and completes the Hello/pairing handshake.
//! 2. Reports its monitor configuration (`ScreenInfo`) so the master can place
//!    it correctly in the virtual layout.
//! 3. Receives `KeyEvent`, `MouseMove`, `MouseButton`, and `MouseScroll`
//!    messages from the master.
//! 4. Translates the platform-independent HID key codes to OS-native codes.
//! 5. Calls the platform input emulation API (`SendInput` on Windows,
//!    XTest on Linux, CoreGraphics on macOS) to inject the events as if the
//!    user were physically typing on the client machine.

/// Application layer: use cases for the client.
pub mod application;

/// Infrastructure layer: OS adapters, network, and UI bridge.
pub mod infrastructure;
