//! Linux X11 input emulation via the XTest extension.
//!
//! Uses `XTestFakeKeyEvent`, `XTestFakeMotionEvent`, and `XTestFakeButtonEvent`
//! to inject input events into the X11 session.
//!
//! # What is XTest? (for beginners)
//!
//! XTest is an X11 protocol extension that lets a process synthesize keyboard
//! and mouse events as if the user had physically interacted with the hardware.
//! These events are delivered to the currently focused window exactly like real
//! input — the receiving application cannot distinguish them from physical input.
//!
//! The key functions are:
//! - `XTestFakeKeyEvent(display, keycode, is_press, time)` — simulate a key
//!   press or release.
//! - `XTestFakeMotionEvent(display, screen, x, y, time)` — move the cursor to
//!   absolute pixel coordinates.
//! - `XTestFakeButtonEvent(display, button, is_press, time)` — simulate a
//!   mouse button press or release.
//!
//! # Key code translation
//!
//! X11 uses *KeySyms* (symbolic names like `XK_a` = 0x0061) rather than
//! physical key positions.  The [`KeyMapper::hid_to_x11_keysym`] function
//! converts the USB HID Usage ID from the protocol into the correct X11 KeySym.
//!
//! Note that `XTestFakeKeyEvent` takes an X11 *keycode* (a hardware scan code
//! mapped by the server), not a KeySym directly.  The conversion is:
//! ```text
//! HID Usage ID → X11 KeySym → XKeysymToKeycode(display, keysym) → X11 keycode
//! ```
//!
//! # Mouse scroll via button events
//!
//! X11 does not have a dedicated scroll-wheel API.  Scroll events are instead
//! represented as button press+release pairs:
//!
//! | Button number | Scroll direction |
//! |--------------|-----------------|
//! | 4            | Up (positive Y) |
//! | 5            | Down (negative Y)|
//! | 6            | Right (positive X)|
//! | 7            | Left (negative X)|
//!
//! The `delta` value from the protocol uses Windows' WHEEL_DELTA = 120 unit
//! per notch convention.  We divide by 120 to get the number of scroll clicks.
//!
//! # Permissions
//!
//! XTest requires the process to have access to the X display.  This is normally
//! satisfied when the process runs in the same user session.  If the `DISPLAY`
//! environment variable is not set or the X server is not accessible, the
//! constructor fails with a `Platform` error.

use kvm_core::{
    keymap::{hid::HidKeyCode, KeyMapper},
    protocol::messages::{ModifierFlags, MouseButton},
};

use crate::application::emulate_input::{EmulationError, PlatformInputEmulator};

// ── X11 constants ─────────────────────────────────────────────────────────────

/// Passing `CurrentTime` (0) to XTest functions means "use the server's current
/// timestamp".  This is the correct value for synthesized events.
const CURRENT_TIME: u64 = 0;

/// Passing `-1` as the screen number to `XTestFakeMotionEvent` means "use the
/// screen that currently contains the pointer".  This is correct for absolute
/// motion events when you want to move within whatever screen the cursor is on.
const SCREEN_DEFAULT: i32 = -1; // Use current screen

/// Linux X11/XTest input emulator.
///
/// In the current state this is a scaffold implementation that validates the
/// key translation path but defers the actual XTest FFI calls.  The production
/// implementation would hold a `*mut x11::xlib::Display` pointer obtained from
/// `XOpenDisplay` and pass it to each XTest call.
pub struct LinuxXTestEmulator {
    // In production, this would hold a raw *mut x11::xlib::Display
    // kept as a placeholder since x11 FFI requires the library at link time
}

impl LinuxXTestEmulator {
    /// Connects to the X display.
    ///
    /// In the production implementation this calls `XOpenDisplay(null)` to
    /// open a connection to the display named by the `DISPLAY` environment
    /// variable.  If `DISPLAY` is not set or the X server is unreachable,
    /// `XOpenDisplay` returns a null pointer and we return an error.
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
        // Translate the USB HID Usage ID to the X11 KeySym for this key.
        let keysym =
            KeyMapper::hid_to_x11_keysym(key).ok_or(EmulationError::InvalidKeyCode(key))?;
        // Production: XTestFakeKeyEvent(display, XKeysymToKeycode(display, keysym), True, CURRENT_TIME)
        // followed by XFlush(display) to ensure the event is sent immediately.
        let _ = keysym;
        Ok(())
    }

    fn emit_key_up(
        &self,
        key: HidKeyCode,
        _modifiers: ModifierFlags,
    ) -> Result<(), EmulationError> {
        let keysym =
            KeyMapper::hid_to_x11_keysym(key).ok_or(EmulationError::InvalidKeyCode(key))?;
        // Production: XTestFakeKeyEvent(display, XKeysymToKeycode(display, keysym), False, CURRENT_TIME)
        // `False` (the third argument) means key-up.
        let _ = keysym;
        Ok(())
    }

    fn emit_mouse_move(&self, x: i32, y: i32) -> Result<(), EmulationError> {
        // Production: XTestFakeMotionEvent(display, SCREEN_DEFAULT, x, y, CURRENT_TIME)
        // followed by XFlush(display).
        // Coordinates are absolute pixel positions on the screen.
        let _ = (x, y);
        let _ = SCREEN_DEFAULT;
        Ok(())
    }

    fn emit_mouse_button(
        &self,
        button: MouseButton,
        pressed: bool,
        _x: i32,
        _y: i32,
    ) -> Result<(), EmulationError> {
        // Map the protocol MouseButton to the X11 button number.
        // X11 button numbering:
        //   1 = Left, 2 = Middle, 3 = Right
        //   8 = Back (browser back button)
        //   9 = Forward (browser forward button)
        let xbutton = match button {
            MouseButton::Left => 1u32,
            MouseButton::Middle => 2,
            MouseButton::Right => 3,
            MouseButton::Button4 => 8,
            MouseButton::Button5 => 9,
        };
        // Production: XTestFakeButtonEvent(display, xbutton, pressed, CURRENT_TIME)
        let _ = (xbutton, pressed);
        let _ = CURRENT_TIME;
        Ok(())
    }

    fn emit_mouse_scroll(&self, delta_x: i16, delta_y: i16) -> Result<(), EmulationError> {
        // X11 encodes scroll events as button press+release pairs.
        // The delta values use the Windows WHEEL_DELTA = 120 convention:
        // one detent of a standard scroll wheel produces +120 or -120.
        // We divide by 120 to convert to scroll click count.

        if delta_y != 0 {
            // Positive delta_y → scroll up (button 4)
            // Negative delta_y → scroll down (button 5)
            let button = if delta_y > 0 { 4u32 } else { 5u32 };
            let clicks = (delta_y.unsigned_abs() / 120).max(1) as usize;
            for _ in 0..clicks {
                // Production: press + release for each click
                let _ = button;
            }
        }
        if delta_x != 0 {
            // Positive delta_x → scroll right (button 7)
            // Negative delta_x → scroll left (button 6)
            let button = if delta_x > 0 { 7u32 } else { 6u32 };
            let clicks = (delta_x.unsigned_abs() / 120).max(1) as usize;
            for _ in 0..clicks {
                let _ = button;
            }
        }
        Ok(())
    }
}
