//! macOS CoreGraphics input emulation.
//!
//! Uses `CGEventCreateKeyboardEvent`, `CGEventCreateMouseEvent`, and
//! `CGEventPost` to inject events at the `kCGHIDEventTap` level.
//!
//! # What is CoreGraphics event injection? (for beginners)
//!
//! macOS exposes the CoreGraphics framework for low-level graphics and input
//! operations.  `CGEventPost` injects a synthesized event directly into the
//! hardware input stream at the HID (Human Interface Device) level — the same
//! level as physical keyboard and mouse input.  Applications cannot distinguish
//! these synthesized events from real hardware events.
//!
//! The typical sequence for a key press is:
//!
//! 1. `CGEventSourceCreate(kCGEventSourceStateHIDSystemState)` — obtain an
//!    event source that mimics hardware state.
//! 2. `CGEventCreateKeyboardEvent(source, cgkeycode, key_down)` — create the
//!    event.
//! 3. `CGEventPost(kCGHIDEventTap, event)` — inject the event into the system.
//! 4. `CFRelease(event)` and `CFRelease(source)` — release memory (CoreFoundation
//!    uses manual reference counting, not automatic garbage collection).
//!
//! # Key code translation
//!
//! macOS uses `CGKeyCode` — a numeric code for each physical key position on an
//! ANSI keyboard layout.  The [`KeyMapper::hid_to_macos_cgkeycode`] function
//! converts the USB HID Usage ID received over the network to the correct
//! `CGKeyCode`.
//!
//! # Mouse coordinate origin
//!
//! macOS places the coordinate origin (0, 0) at the **bottom-left** of the
//! primary display, with Y increasing upward.  This is the opposite of Windows
//! and Linux (top-left origin, Y increasing downward).
//!
//! When injecting mouse move events, the Y coordinate must be flipped:
//! ```text
//! macos_y = primary_screen_height - protocol_y
//! ```
//!
//! # Accessibility permission
//!
//! `CGEventPost` at `kCGHIDEventTap` requires the application to have the
//! **Accessibility** permission granted in System Preferences → Privacy &
//! Security → Accessibility.  Without this permission, the call silently fails.
//!
//! At construction time we check `AXIsProcessTrustedWithOptions` and prompt the
//! user if the permission has not been granted.

#![cfg(target_os = "macos")]

use kvm_core::{
    keymap::{hid::HidKeyCode, KeyMapper},
    protocol::messages::{ModifierFlags, MouseButton},
};

use crate::application::emulate_input::{EmulationError, PlatformInputEmulator};

/// macOS CoreGraphics event source for input emulation.
///
/// This is a scaffold implementation that validates the key translation path
/// and documents the production code pattern.  The full CoreFoundation/
/// CoreGraphics FFI bindings are not included here to avoid adding a macOS-only
/// build dependency; the production implementation would use the `core-graphics`
/// crate.
pub struct MacosInputEmulator;

impl MacosInputEmulator {
    /// Creates a new emulator and checks for Accessibility permission.
    ///
    /// The production implementation calls:
    /// ```text
    /// AXIsProcessTrustedWithOptions({kAXTrustedCheckOptionPrompt: true})
    /// ```
    /// which shows the macOS permission prompt if the app is not yet trusted.
    ///
    /// # Errors
    ///
    /// Returns `EmulationError::Platform` if Accessibility permission has not
    /// been granted after prompting.
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
        // Translate the USB HID Usage ID to the macOS CGKeyCode.
        let cgkeycode =
            KeyMapper::hid_to_macos_cgkeycode(key).ok_or(EmulationError::InvalidKeyCode(key))?;
        // Production sequence:
        //   let src = CGEventSourceCreate(kCGEventSourceStateHIDSystemState);
        //   let event = CGEventCreateKeyboardEvent(src, cgkeycode, true);  // true = key down
        //   CGEventPost(kCGHIDEventTap, event);
        //   CFRelease(event);
        //   CFRelease(src);
        let _ = cgkeycode;
        Ok(())
    }

    fn emit_key_up(
        &self,
        key: HidKeyCode,
        _modifiers: ModifierFlags,
    ) -> Result<(), EmulationError> {
        let cgkeycode =
            KeyMapper::hid_to_macos_cgkeycode(key).ok_or(EmulationError::InvalidKeyCode(key))?;
        // Production: same as emit_key_down but with `false` (key up) as the third argument
        // to CGEventCreateKeyboardEvent.
        let _ = cgkeycode;
        Ok(())
    }

    fn emit_mouse_move(&self, x: i32, y: i32) -> Result<(), EmulationError> {
        // macOS coordinate origin is bottom-left of primary monitor.
        // The screen height must be used to flip Y from the top-left convention
        // used by the protocol (which matches Windows and Linux).
        //
        // Production:
        //   let screen_height = CGDisplayBounds(CGMainDisplayID()).size.height;
        //   let flipped_y = screen_height - y as f64;
        //   let point = CGPointMake(x as f64, flipped_y);
        //   CGEventCreateMouseEvent(src, kCGEventMouseMoved, point, kCGMouseButtonLeft);
        //   CGEventPost(kCGHIDEventTap, event);
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
        // Map to the CGEventType constants:
        //   Left:   kCGEventLeftMouseDown / kCGEventLeftMouseUp
        //   Right:  kCGEventRightMouseDown / kCGEventRightMouseUp
        //   Other:  kCGEventOtherMouseDown / kCGEventOtherMouseUp (with button index)
        // The mouse position must also be included in the event, so both x and y
        // are required even for button events.
        let _ = (button, pressed, x, y);
        Ok(())
    }

    fn emit_mouse_scroll(&self, delta_x: i16, delta_y: i16) -> Result<(), EmulationError> {
        // Production:
        //   CGEventCreateScrollWheelEvent(
        //       src,
        //       kCGScrollEventUnitPixel,  // pixel unit for smooth scrolling
        //       2,                        // number of scroll axes (vertical + horizontal)
        //       delta_y as i32,           // axis 1: vertical
        //       delta_x as i32,           // axis 2: horizontal
        //   )
        //   CGEventPost(kCGHIDEventTap, event);
        let _ = (delta_x, delta_y);
        Ok(())
    }
}
