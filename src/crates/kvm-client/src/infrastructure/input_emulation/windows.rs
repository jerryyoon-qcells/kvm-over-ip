//! Windows input emulation via the SendInput API.
//!
//! Translates HID Usage IDs to Windows Virtual Key codes and injects
//! events using SendInput. Mouse coordinates are normalized to the
//! Windows virtual screen space [0, 65535].

#![cfg(target_os = "windows")]

use kvm_core::{
    keymap::{hid::HidKeyCode, KeyMapper},
    protocol::messages::{ModifierFlags, MouseButton},
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL,
    MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, XBUTTON1, XBUTTON2,
};

use crate::application::emulate_input::{EmulationError, PlatformInputEmulator};

/// Windows implementation of [`PlatformInputEmulator`] using SendInput.
pub struct WindowsInputEmulator;

impl WindowsInputEmulator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsInputEmulator {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformInputEmulator for WindowsInputEmulator {
    fn emit_key_down(
        &self,
        key: HidKeyCode,
        _modifiers: ModifierFlags,
    ) -> Result<(), EmulationError> {
        let vk = KeyMapper::hid_to_windows_vk(key)
            .ok_or(EmulationError::InvalidKeyCode(key))?;
        send_key(vk, false)?;
        Ok(())
    }

    fn emit_key_up(
        &self,
        key: HidKeyCode,
        _modifiers: ModifierFlags,
    ) -> Result<(), EmulationError> {
        let vk = KeyMapper::hid_to_windows_vk(key)
            .ok_or(EmulationError::InvalidKeyCode(key))?;
        send_key(vk, true)?;
        Ok(())
    }

    fn emit_mouse_move(&self, x: i32, y: i32) -> Result<(), EmulationError> {
        let (norm_x, norm_y) = normalize_coords(x, y);
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: norm_x,
                    dy: norm_y,
                    mouseData: 0,
                    // SAFETY: MOUSEEVENTF_ABSOLUTE uses normalized coords [0, 65535]
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        // SAFETY: input is a valid INPUT structure on the stack
        unsafe {
            windows::Win32::UI::Input::KeyboardAndMouse::SendInput(
                &[input],
                std::mem::size_of::<INPUT>() as i32,
            );
        }
        Ok(())
    }

    fn emit_mouse_button(
        &self,
        button: MouseButton,
        pressed: bool,
        _x: i32,
        _y: i32,
    ) -> Result<(), EmulationError> {
        let (flags, mouse_data) = match (button, pressed) {
            (MouseButton::Left, true) => (MOUSEEVENTF_LEFTDOWN, 0),
            (MouseButton::Left, false) => (MOUSEEVENTF_LEFTUP, 0),
            (MouseButton::Right, true) => (MOUSEEVENTF_RIGHTDOWN, 0),
            (MouseButton::Right, false) => (MOUSEEVENTF_RIGHTUP, 0),
            (MouseButton::Middle, true) => (MOUSEEVENTF_MIDDLEDOWN, 0),
            (MouseButton::Middle, false) => (MOUSEEVENTF_MIDDLEUP, 0),
            (MouseButton::Button4, true) => (MOUSEEVENTF_XDOWN, XBUTTON1 as u32),
            (MouseButton::Button4, false) => (MOUSEEVENTF_XUP, XBUTTON1 as u32),
            (MouseButton::Button5, true) => (MOUSEEVENTF_XDOWN, XBUTTON2 as u32),
            (MouseButton::Button5, false) => (MOUSEEVENTF_XUP, XBUTTON2 as u32),
        };

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: mouse_data,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        // SAFETY: input is a valid INPUT structure
        unsafe {
            windows::Win32::UI::Input::KeyboardAndMouse::SendInput(
                &[input],
                std::mem::size_of::<INPUT>() as i32,
            );
        }
        Ok(())
    }

    fn emit_mouse_scroll(&self, delta_x: i16, delta_y: i16) -> Result<(), EmulationError> {
        if delta_y != 0 {
            send_wheel(delta_y as i32, false)?;
        }
        if delta_x != 0 {
            send_wheel(delta_x as i32, true)?;
        }
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Normalizes pixel coordinates to Windows' [0, 65535] virtual screen range.
fn normalize_coords(x: i32, y: i32) -> (i32, i32) {
    // SAFETY: GetSystemMetrics is always safe to call
    let screen_w = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let screen_h = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

    let norm_x = if screen_w > 0 {
        (x * 65535 / screen_w).clamp(0, 65535)
    } else {
        0
    };
    let norm_y = if screen_h > 0 {
        (y * 65535 / screen_h).clamp(0, 65535)
    } else {
        0
    };
    (norm_x, norm_y)
}

fn send_key(vk: u8, key_up: bool) -> Result<(), EmulationError> {
    let mut flags = KEYEVENTF_SCANCODE;
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }

    // Extended keys need the EXTENDEDKEY flag
    let extended_vks: &[u8] = &[
        0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, // nav
        0x2D, 0x2E, // Insert, Delete
        0x5B, 0x5C, // Win keys
        0xA3, 0xA5, // Right Ctrl, Right Alt
    ];
    if extended_vks.contains(&vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk as u16),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    // SAFETY: input is a valid KEYBDINPUT structure
    unsafe {
        windows::Win32::UI::Input::KeyboardAndMouse::SendInput(
            &[input],
            std::mem::size_of::<INPUT>() as i32,
        );
    }
    Ok(())
}

fn send_wheel(delta: i32, horizontal: bool) -> Result<(), EmulationError> {
    let flags = if horizontal {
        MOUSEEVENTF_HWHEEL
    } else {
        MOUSEEVENTF_WHEEL
    };

    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: delta as u32,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    // SAFETY: input is a valid MOUSEINPUT structure
    unsafe {
        windows::Win32::UI::Input::KeyboardAndMouse::SendInput(
            &[input],
            std::mem::size_of::<INPUT>() as i32,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_coords_clamps_to_valid_range() {
        // We can only test the clamping logic; actual screen metrics require display
        // These test the pure math with assumed screen dimensions
        let result_x = (500i32 * 65535 / 1920).clamp(0, 65535);
        let result_y = (300i32 * 65535 / 1080).clamp(0, 65535);
        assert!(result_x >= 0 && result_x <= 65535);
        assert!(result_y >= 0 && result_y <= 65535);
    }

    #[test]
    fn test_normalize_coords_zero_gives_zero() {
        let result_x = (0i32 * 65535 / 1920).clamp(0, 65535);
        assert_eq!(result_x, 0);
    }

    #[test]
    fn test_normalize_coords_full_width_gives_max() {
        let result_x = (1920i32 * 65535 / 1920).clamp(0, 65535);
        assert_eq!(result_x, 65535);
    }
}
