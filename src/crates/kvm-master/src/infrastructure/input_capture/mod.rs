//! Input capture infrastructure for the master application.
//!
//! On Windows, this installs low-level keyboard and mouse hooks (WH_KEYBOARD_LL,
//! WH_MOUSE_LL) on a dedicated Win32 message loop thread. Raw events are placed
//! into a lock-free channel and consumed by the Tokio async runtime.
//!
//! # Windows-Specific Implementation
//!
//! The hook callbacks must complete within ~300ms or Windows will remove the hook.
//! All processing is deferred out of the callback via an `mpsc` channel.
//!
//! # Testability
//!
//! The `InputSource` trait allows unit tests to inject synthetic events without
//! requiring Windows hooks.

use std::sync::mpsc;

pub mod mock;

#[cfg(target_os = "windows")]
pub mod windows;

/// A raw input event produced by the input capture infrastructure.
#[derive(Debug, Clone)]
pub enum RawInputEvent {
    /// A key was pressed down.
    KeyDown {
        /// Windows Virtual Key code.
        vk_code: u8,
        /// Hardware scan code.
        scan_code: u16,
        /// Milliseconds since system start (from the hook struct).
        time_ms: u32,
        /// `true` if this is an extended key (e.g., right-side modifiers, numpad Enter).
        is_extended: bool,
    },
    /// A key was released.
    KeyUp {
        vk_code: u8,
        scan_code: u16,
        time_ms: u32,
        is_extended: bool,
    },
    /// The mouse cursor moved to an absolute screen position.
    MouseMove {
        /// Absolute X in virtual screen coordinates (multi-monitor aware).
        x: i32,
        /// Absolute Y in virtual screen coordinates.
        y: i32,
        time_ms: u32,
    },
    /// A mouse button was pressed.
    MouseButtonDown {
        button: MouseButton,
        x: i32,
        y: i32,
        time_ms: u32,
    },
    /// A mouse button was released.
    MouseButtonUp {
        button: MouseButton,
        x: i32,
        y: i32,
        time_ms: u32,
    },
    /// The vertical mouse wheel was scrolled.
    MouseWheel {
        /// Scroll delta; positive = away from user, negative = toward user.
        delta: i16,
        x: i32,
        y: i32,
        time_ms: u32,
    },
    /// The horizontal mouse wheel was scrolled.
    MouseWheelH {
        /// Scroll delta; positive = right, negative = left.
        delta: i16,
        x: i32,
        y: i32,
        time_ms: u32,
    },
}

/// Mouse button identifier used in [`RawInputEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

/// Error type for input capture operations.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("failed to install keyboard hook: {0}")]
    KeyboardHookInstallFailed(String),
    #[error("failed to install mouse hook: {0}")]
    MouseHookInstallFailed(String),
    #[error("capture service has already been stopped")]
    AlreadyStopped,
    #[error("platform not supported: {0}")]
    UnsupportedPlatform(String),
}

/// Trait abstracting input event production.
///
/// The production implementation uses Windows hooks; tests use [`mock::MockInputSource`].
pub trait InputSource: Send {
    /// Starts the input source and returns a receiver for captured events.
    fn start(&self) -> Result<mpsc::Receiver<RawInputEvent>, CaptureError>;
    /// Stops the input source and releases all OS resources.
    fn stop(&self);
    /// Instructs the source to suppress the current event (if applicable).
    fn suppress_current_event(&self);
}
