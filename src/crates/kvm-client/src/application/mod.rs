//! Application layer use cases for the client application.
//!
//! # What use cases does the client have?
//!
//! - **`emulate_input`** – Translates received `KvmMessage` events (which use
//!   platform-independent HID key codes) into OS-native input calls.  The
//!   actual OS call is made by a `PlatformInputEmulator` implementation that
//!   is injected at construction time.
//!
//! - **`report_screens`** – Enumerates the client's physical monitors and
//!   formats the information for the `ScreenInfo` message that is sent to the
//!   master after connecting.  The master uses this to correctly size the
//!   client screen in the virtual layout editor.

pub mod emulate_input;
pub mod report_screens;
