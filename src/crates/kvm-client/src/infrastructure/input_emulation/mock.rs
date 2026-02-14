//! Mock platform input emulator for unit testing.
//!
//! # Why a mock emulator?
//!
//! The real input emulators (`WindowsInputEmulator`, `LinuxXTestEmulator`,
//! `MacosInputEmulator`) make OS API calls that:
//!
//! - Require a physical desktop environment to run.
//! - Actually move the cursor or press keys on the test machine.
//! - Cannot be observed directly from Rust test code.
//!
//! The `MockInputEmulator` replaces all OS calls with simple in-memory
//! recording.  Each emitted event is pushed into a `Mutex<Vec<...>>` so that
//! test assertions can inspect exactly what was emitted and in what order.
//!
//! # Usage in tests
//!
//! ```ignore
//! let emulator = Arc::new(MockInputEmulator::new());
//! let use_case = EmulateInputUseCase::new(Arc::clone(&emulator));
//!
//! use_case.handle_key_event(&key_down_event).unwrap();
//!
//! // Assert that exactly one key-down event was recorded.
//! let downs = emulator.key_downs.lock().unwrap();
//! assert_eq!(downs.len(), 1);
//! assert_eq!(downs[0].0, HidKeyCode::KeyA);
//! ```
//!
//! # `should_fail` flag
//!
//! Set `should_fail = true` before calling a method to simulate OS failures.
//! This lets you test error-handling paths in the use case without needing a
//! broken OS.

use std::sync::Mutex;

use kvm_core::{
    keymap::hid::HidKeyCode,
    protocol::messages::{ModifierFlags, MouseButton},
};

use crate::application::emulate_input::{EmulationError, PlatformInputEmulator};

/// A mock emulator that records all calls without performing OS API calls.
///
/// All event records are stored in `Mutex<Vec<...>>` fields so tests can safely
/// share the emulator across threads (e.g., when wrapping it in an `Arc`).
#[derive(Default)]
pub struct MockInputEmulator {
    /// Records each (key, modifiers) pair passed to `emit_key_down`.
    pub key_downs: Mutex<Vec<(HidKeyCode, ModifierFlags)>>,
    /// Records each (key, modifiers) pair passed to `emit_key_up`.
    pub key_ups: Mutex<Vec<(HidKeyCode, ModifierFlags)>>,
    /// Records each (x, y) pixel position passed to `emit_mouse_move`.
    pub mouse_moves: Mutex<Vec<(i32, i32)>>,
    /// Records (button, pressed, x, y) tuples from `emit_mouse_button`.
    pub mouse_buttons: Mutex<Vec<(MouseButton, bool, i32, i32)>>,
    /// Records (delta_x, delta_y) pairs from `emit_mouse_scroll`.
    pub scrolls: Mutex<Vec<(i16, i16)>>,
    /// When `true`, every method immediately returns an `EmulationError::Platform`.
    /// Use this to test error-handling paths in callers.
    pub should_fail: bool,
}

impl MockInputEmulator {
    /// Creates a new `MockInputEmulator` with empty records and `should_fail = false`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl PlatformInputEmulator for MockInputEmulator {
    /// Records the key-down event, or returns an error if `should_fail` is set.
    fn emit_key_down(
        &self,
        key: HidKeyCode,
        modifiers: ModifierFlags,
    ) -> Result<(), EmulationError> {
        if self.should_fail {
            return Err(EmulationError::Platform("mock failure".into()));
        }
        self.key_downs.lock().unwrap().push((key, modifiers));
        Ok(())
    }

    /// Records the key-up event, or returns an error if `should_fail` is set.
    fn emit_key_up(
        &self,
        key: HidKeyCode,
        modifiers: ModifierFlags,
    ) -> Result<(), EmulationError> {
        if self.should_fail {
            return Err(EmulationError::Platform("mock failure".into()));
        }
        self.key_ups.lock().unwrap().push((key, modifiers));
        Ok(())
    }

    /// Records the mouse position, or returns an error if `should_fail` is set.
    fn emit_mouse_move(&self, x: i32, y: i32) -> Result<(), EmulationError> {
        if self.should_fail {
            return Err(EmulationError::Platform("mock failure".into()));
        }
        self.mouse_moves.lock().unwrap().push((x, y));
        Ok(())
    }

    /// Records the mouse button event, or returns an error if `should_fail` is set.
    fn emit_mouse_button(
        &self,
        button: MouseButton,
        pressed: bool,
        x: i32,
        y: i32,
    ) -> Result<(), EmulationError> {
        if self.should_fail {
            return Err(EmulationError::Platform("mock failure".into()));
        }
        self.mouse_buttons.lock().unwrap().push((button, pressed, x, y));
        Ok(())
    }

    /// Records the scroll delta, or returns an error if `should_fail` is set.
    fn emit_mouse_scroll(&self, delta_x: i16, delta_y: i16) -> Result<(), EmulationError> {
        if self.should_fail {
            return Err(EmulationError::Platform("mock failure".into()));
        }
        self.scrolls.lock().unwrap().push((delta_x, delta_y));
        Ok(())
    }
}
