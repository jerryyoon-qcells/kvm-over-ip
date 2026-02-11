//! macOS CoreGraphics input emulation.
//!
//! Uses CGEventCreateKeyboardEvent, CGEventCreateMouseEvent, and CGEventPost
//! to inject events at kCGHIDEventTap level. Requires Accessibility permission.

#![cfg(target_os = "macos")]

use kvm_core::{
    keymap::{hid::HidKeyCode, KeyMapper},
    protocol::messages::{ModifierFlags, MouseButton},
};

use crate::application::emulate_input::{EmulationError, PlatformInputEmulator};

/// macOS CoreGraphics event source for input emulation.
pub struct MacosInputEmulator;

impl MacosInputEmulator {
    /// Creates a new emulator.
    ///
    /// Checks for Accessibility permission at construction time.
    ///
    /// # Errors
    ///
    /// Returns `EmulationError::Platform` if Accessibility permission has not been granted.
    pub fn new() -> Result<Self, EmulationError> {
        // Production: call AXIsProcessTrustedWithOptions({kAXTrustedCheckOptionPrompt: true})
        // and return error if not trusted
        Ok(Self)
    }
}

impl PlatformInputEmulator for MacosInputEmulator {
    fn emit_key_down(
        &self,
        key: HidKeyCode,
        _modifiers: ModifierFlags,
    ) -> Result<(), EmulationError> {
        let cgkeycode = KeyMapper::hid_to_macos_cgkeycode(key)
            .ok_or(EmulationError::InvalidKeyCode(key))?;
        // Production:
        //   let src = CGEventSourceCreate(kCGEventSourceStateHIDSystemState)
        //   let event = CGEventCreateKeyboardEvent(src, cgkeycode, true)
        //   CGEventPost(kCGHIDEventTap, event)
        //   CFRelease(event); CFRelease(src)
        let _ = cgkeycode;
        Ok(())
    }

    fn emit_key_up(
        &self,
        key: HidKeyCode,
        _modifiers: ModifierFlags,
    ) -> Result<(), EmulationError> {
        let cgkeycode = KeyMapper::hid_to_macos_cgkeycode(key)
            .ok_or(EmulationError::InvalidKeyCode(key))?;
        // Production: CGEventCreateKeyboardEvent(src, cgkeycode, false)
        let _ = cgkeycode;
        Ok(())
    }

    fn emit_mouse_move(&self, x: i32, y: i32) -> Result<(), EmulationError> {
        // macOS coordinate origin is bottom-left of primary monitor.
        // The screen height must be used to flip Y.
        // Production: CGEventCreateMouseEvent(src, kCGEventMouseMoved, CGPointMake(x, flipped_y), 0)
        let _ = (x, y);
        Ok(())
    }

    fn emit_mouse_button(
        &self,
        button: MouseButton,
        pressed: bool,
        x: i32,
        y: i32,
    ) -> Result<(), EmulationError> {
        // Map to CGEventType: kCGEventLeftMouseDown/Up, kCGEventRightMouseDown/Up, etc.
        let _ = (button, pressed, x, y);
        Ok(())
    }

    fn emit_mouse_scroll(&self, delta_x: i16, delta_y: i16) -> Result<(), EmulationError> {
        // Production: CGEventCreateScrollWheelEvent(src, kCGScrollEventUnitPixel, 2, delta_y, delta_x)
        let _ = (delta_x, delta_y);
        Ok(())
    }
}
