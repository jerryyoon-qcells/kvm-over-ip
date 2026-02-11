//! Mock platform input emulator for unit testing.

use std::sync::Mutex;

use kvm_core::{
    keymap::hid::HidKeyCode,
    protocol::messages::{ModifierFlags, MouseButton},
};

use crate::application::emulate_input::{EmulationError, PlatformInputEmulator};

/// A mock emulator that records all calls without performing OS API calls.
#[derive(Default)]
pub struct MockInputEmulator {
    pub key_downs: Mutex<Vec<(HidKeyCode, ModifierFlags)>>,
    pub key_ups: Mutex<Vec<(HidKeyCode, ModifierFlags)>>,
    pub mouse_moves: Mutex<Vec<(i32, i32)>>,
    pub mouse_buttons: Mutex<Vec<(MouseButton, bool, i32, i32)>>,
    pub scrolls: Mutex<Vec<(i16, i16)>>,
    pub should_fail: bool,
}

impl MockInputEmulator {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PlatformInputEmulator for MockInputEmulator {
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

    fn emit_mouse_move(&self, x: i32, y: i32) -> Result<(), EmulationError> {
        if self.should_fail {
            return Err(EmulationError::Platform("mock failure".into()));
        }
        self.mouse_moves.lock().unwrap().push((x, y));
        Ok(())
    }

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

    fn emit_mouse_scroll(&self, delta_x: i16, delta_y: i16) -> Result<(), EmulationError> {
        if self.should_fail {
            return Err(EmulationError::Platform("mock failure".into()));
        }
        self.scrolls.lock().unwrap().push((delta_x, delta_y));
        Ok(())
    }
}
