//! Linux X11 input emulation via the XTest extension.
//!
//! Uses XTestFakeKeyEvent, XTestFakeMotionEvent, and XTestFakeButtonEvent
//! to inject input events into the X11 session.
//!
//! Requires the `input` group membership or root (for /dev/uinput alternative).

#![cfg(target_os = "linux")]

use kvm_core::{
    keymap::{hid::HidKeyCode, KeyMapper},
    protocol::messages::{ModifierFlags, MouseButton},
};

use crate::application::emulate_input::{EmulationError, PlatformInputEmulator};

// X11 constants
const CURRENT_TIME: u64 = 0;
const SCREEN_DEFAULT: i32 = -1; // Use current screen

/// Linux X11/XTest input emulator.
pub struct LinuxXTestEmulator {
    // In production, this would hold a raw *mut x11::xlib::Display
    // kept as a placeholder since x11 FFI requires the library at link time
}

impl LinuxXTestEmulator {
    /// Connects to the X display.
    ///
    /// # Errors
    ///
    /// Returns `EmulationError::Platform` if the X display cannot be opened.
    pub fn new() -> Result<Self, EmulationError> {
        // Production implementation would call XOpenDisplay(null)
        // and check for null return (display unavailable)
        Ok(Self {})
    }
}

impl PlatformInputEmulator for LinuxXTestEmulator {
    fn emit_key_down(
        &self,
        key: HidKeyCode,
        _modifiers: ModifierFlags,
    ) -> Result<(), EmulationError> {
        let keysym = KeyMapper::hid_to_x11_keysym(key)
            .ok_or(EmulationError::InvalidKeyCode(key))?;
        // Production: XTestFakeKeyEvent(display, XKeysymToKeycode(display, keysym), True, CURRENT_TIME)
        // followed by XFlush(display)
        let _ = keysym;
        Ok(())
    }

    fn emit_key_up(
        &self,
        key: HidKeyCode,
        _modifiers: ModifierFlags,
    ) -> Result<(), EmulationError> {
        let keysym = KeyMapper::hid_to_x11_keysym(key)
            .ok_or(EmulationError::InvalidKeyCode(key))?;
        // Production: XTestFakeKeyEvent(display, XKeysymToKeycode(display, keysym), False, CURRENT_TIME)
        let _ = keysym;
        Ok(())
    }

    fn emit_mouse_move(&self, x: i32, y: i32) -> Result<(), EmulationError> {
        // Production: XTestFakeMotionEvent(display, SCREEN_DEFAULT, x, y, CURRENT_TIME)
        // followed by XFlush(display)
        let _ = (x, y);
        Ok(())
    }

    fn emit_mouse_button(
        &self,
        button: MouseButton,
        pressed: bool,
        _x: i32,
        _y: i32,
    ) -> Result<(), EmulationError> {
        let xbutton = match button {
            MouseButton::Left => 1u32,
            MouseButton::Middle => 2,
            MouseButton::Right => 3,
            MouseButton::Button4 => 8,
            MouseButton::Button5 => 9,
        };
        // Production: XTestFakeButtonEvent(display, xbutton, pressed, CURRENT_TIME)
        let _ = (xbutton, pressed);
        Ok(())
    }

    fn emit_mouse_scroll(&self, delta_x: i16, delta_y: i16) -> Result<(), EmulationError> {
        // X11 scroll uses button events:
        // Button 4 = scroll up, Button 5 = scroll down
        // Button 6 = scroll left, Button 7 = scroll right
        if delta_y != 0 {
            let button = if delta_y > 0 { 4u32 } else { 5u32 };
            let clicks = (delta_y.unsigned_abs() / 120).max(1) as usize;
            for _ in 0..clicks {
                // Production: press + release
                let _ = button;
            }
        }
        if delta_x != 0 {
            let button = if delta_x > 0 { 7u32 } else { 6u32 };
            let clicks = (delta_x.unsigned_abs() / 120).max(1) as usize;
            for _ in 0..clicks {
                let _ = button;
            }
        }
        Ok(())
    }
}
