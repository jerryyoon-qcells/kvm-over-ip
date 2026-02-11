//! HID Usage ID to macOS CGKeyCode translation table.
//!
//! CGKeyCode values are defined in Carbon Events.h (HIToolbox framework).
//! Reference: https://github.com/nicholasess/mac-keycode-list/blob/master/keycode.json
//! and /System/Library/Frameworks/Carbon.framework/Versions/A/Frameworks/HIToolbox.framework/Headers/Events.h

use super::hid::HidKeyCode;

/// Translates a [`HidKeyCode`] to a macOS `CGKeyCode` value.
///
/// Returns `None` if the HID code has no macOS CGKeyCode equivalent.
///
/// # Panics
///
/// This function never panics.
pub fn hid_to_cgkeycode(hid: HidKeyCode) -> Option<u16> {
    match hid {
        // Letters (macOS uses ANSI key position codes, not ASCII)
        HidKeyCode::KeyA => Some(0x00), // kVK_ANSI_A
        HidKeyCode::KeyB => Some(0x0B), // kVK_ANSI_B
        HidKeyCode::KeyC => Some(0x08), // kVK_ANSI_C
        HidKeyCode::KeyD => Some(0x02), // kVK_ANSI_D
        HidKeyCode::KeyE => Some(0x0E), // kVK_ANSI_E
        HidKeyCode::KeyF => Some(0x03), // kVK_ANSI_F
        HidKeyCode::KeyG => Some(0x05), // kVK_ANSI_G
        HidKeyCode::KeyH => Some(0x04), // kVK_ANSI_H
        HidKeyCode::KeyI => Some(0x22), // kVK_ANSI_I
        HidKeyCode::KeyJ => Some(0x26), // kVK_ANSI_J
        HidKeyCode::KeyK => Some(0x28), // kVK_ANSI_K
        HidKeyCode::KeyL => Some(0x25), // kVK_ANSI_L
        HidKeyCode::KeyM => Some(0x2E), // kVK_ANSI_M
        HidKeyCode::KeyN => Some(0x2D), // kVK_ANSI_N
        HidKeyCode::KeyO => Some(0x1F), // kVK_ANSI_O
        HidKeyCode::KeyP => Some(0x23), // kVK_ANSI_P
        HidKeyCode::KeyQ => Some(0x0C), // kVK_ANSI_Q
        HidKeyCode::KeyR => Some(0x0F), // kVK_ANSI_R
        HidKeyCode::KeyS => Some(0x01), // kVK_ANSI_S
        HidKeyCode::KeyT => Some(0x11), // kVK_ANSI_T
        HidKeyCode::KeyU => Some(0x20), // kVK_ANSI_U
        HidKeyCode::KeyV => Some(0x09), // kVK_ANSI_V
        HidKeyCode::KeyW => Some(0x0D), // kVK_ANSI_W
        HidKeyCode::KeyX => Some(0x07), // kVK_ANSI_X
        HidKeyCode::KeyY => Some(0x10), // kVK_ANSI_Y
        HidKeyCode::KeyZ => Some(0x06), // kVK_ANSI_Z

        // Digits
        HidKeyCode::Digit0 => Some(0x1D), // kVK_ANSI_0
        HidKeyCode::Digit1 => Some(0x12), // kVK_ANSI_1
        HidKeyCode::Digit2 => Some(0x13), // kVK_ANSI_2
        HidKeyCode::Digit3 => Some(0x14), // kVK_ANSI_3
        HidKeyCode::Digit4 => Some(0x15), // kVK_ANSI_4
        HidKeyCode::Digit5 => Some(0x17), // kVK_ANSI_5
        HidKeyCode::Digit6 => Some(0x16), // kVK_ANSI_6
        HidKeyCode::Digit7 => Some(0x1A), // kVK_ANSI_7
        HidKeyCode::Digit8 => Some(0x1C), // kVK_ANSI_8
        HidKeyCode::Digit9 => Some(0x19), // kVK_ANSI_9

        // Control keys
        HidKeyCode::Enter => Some(0x24),      // kVK_Return
        HidKeyCode::Escape => Some(0x35),     // kVK_Escape
        HidKeyCode::Backspace => Some(0x33),  // kVK_Delete
        HidKeyCode::Tab => Some(0x30),        // kVK_Tab
        HidKeyCode::Space => Some(0x31),      // kVK_Space
        HidKeyCode::CapsLock => Some(0x39),   // kVK_CapsLock
        HidKeyCode::ScrollLock => Some(0x6B), // kVK_F14 (mapped to ScrollLock)
        HidKeyCode::Pause => Some(0x71),      // kVK_F15 (mapped to Pause)
        HidKeyCode::Insert => Some(0x72),     // kVK_Help (insert on some keyboards)
        HidKeyCode::Home => Some(0x73),       // kVK_Home
        HidKeyCode::PageUp => Some(0x74),     // kVK_PageUp
        HidKeyCode::Delete => Some(0x75),     // kVK_ForwardDelete
        HidKeyCode::End => Some(0x77),        // kVK_End
        HidKeyCode::PageDown => Some(0x79),   // kVK_PageDown
        HidKeyCode::PrintScreen => Some(0x69), // kVK_F13
        HidKeyCode::ContextMenu => Some(0x6E), // (no direct equivalent; using kVK_F15 region)

        // Arrow keys
        HidKeyCode::ArrowLeft => Some(0x7B),  // kVK_LeftArrow
        HidKeyCode::ArrowRight => Some(0x7C), // kVK_RightArrow
        HidKeyCode::ArrowDown => Some(0x7D),  // kVK_DownArrow
        HidKeyCode::ArrowUp => Some(0x7E),    // kVK_UpArrow

        // Function keys
        HidKeyCode::F1 => Some(0x7A),  // kVK_F1
        HidKeyCode::F2 => Some(0x78),  // kVK_F2
        HidKeyCode::F3 => Some(0x63),  // kVK_F3
        HidKeyCode::F4 => Some(0x76),  // kVK_F4
        HidKeyCode::F5 => Some(0x60),  // kVK_F5
        HidKeyCode::F6 => Some(0x61),  // kVK_F6
        HidKeyCode::F7 => Some(0x62),  // kVK_F7
        HidKeyCode::F8 => Some(0x64),  // kVK_F8
        HidKeyCode::F9 => Some(0x65),  // kVK_F9
        HidKeyCode::F10 => Some(0x6D), // kVK_F10
        HidKeyCode::F11 => Some(0x67), // kVK_F11
        HidKeyCode::F12 => Some(0x6F), // kVK_F12

        // Numpad
        HidKeyCode::NumLock => Some(0x47),        // kVK_ANSI_KeypadClear (NumLock on Mac)
        HidKeyCode::NumpadDivide => Some(0x4B),   // kVK_ANSI_KeypadDivide
        HidKeyCode::NumpadMultiply => Some(0x43), // kVK_ANSI_KeypadMultiply
        HidKeyCode::NumpadSubtract => Some(0x4E), // kVK_ANSI_KeypadMinus
        HidKeyCode::NumpadAdd => Some(0x45),      // kVK_ANSI_KeypadPlus
        HidKeyCode::NumpadEnter => Some(0x4C),    // kVK_ANSI_KeypadEnter
        HidKeyCode::Numpad0 => Some(0x52),        // kVK_ANSI_Keypad0
        HidKeyCode::Numpad1 => Some(0x53),        // kVK_ANSI_Keypad1
        HidKeyCode::Numpad2 => Some(0x54),        // kVK_ANSI_Keypad2
        HidKeyCode::Numpad3 => Some(0x55),        // kVK_ANSI_Keypad3
        HidKeyCode::Numpad4 => Some(0x56),        // kVK_ANSI_Keypad4
        HidKeyCode::Numpad5 => Some(0x57),        // kVK_ANSI_Keypad5
        HidKeyCode::Numpad6 => Some(0x58),        // kVK_ANSI_Keypad6
        HidKeyCode::Numpad7 => Some(0x59),        // kVK_ANSI_Keypad7
        HidKeyCode::Numpad8 => Some(0x5B),        // kVK_ANSI_Keypad8
        HidKeyCode::Numpad9 => Some(0x5C),        // kVK_ANSI_Keypad9
        HidKeyCode::NumpadDecimal => Some(0x41),  // kVK_ANSI_KeypadDecimal

        // Punctuation / symbols
        HidKeyCode::Minus => Some(0x1B),        // kVK_ANSI_Minus
        HidKeyCode::Equal => Some(0x18),        // kVK_ANSI_Equal
        HidKeyCode::BracketLeft => Some(0x21),  // kVK_ANSI_LeftBracket
        HidKeyCode::BracketRight => Some(0x1E), // kVK_ANSI_RightBracket
        HidKeyCode::Backslash => Some(0x2A),    // kVK_ANSI_Backslash
        HidKeyCode::Semicolon => Some(0x29),    // kVK_ANSI_Semicolon
        HidKeyCode::Quote => Some(0x27),        // kVK_ANSI_Quote
        HidKeyCode::Backquote => Some(0x32),    // kVK_ANSI_Grave
        HidKeyCode::Comma => Some(0x2B),        // kVK_ANSI_Comma
        HidKeyCode::Period => Some(0x2F),       // kVK_ANSI_Period
        HidKeyCode::Slash => Some(0x2C),        // kVK_ANSI_Slash

        // Modifier keys
        HidKeyCode::ControlLeft => Some(0x3B),  // kVK_Control
        HidKeyCode::ControlRight => Some(0x3E), // kVK_RightControl
        HidKeyCode::ShiftLeft => Some(0x38),    // kVK_Shift
        HidKeyCode::ShiftRight => Some(0x3C),   // kVK_RightShift
        HidKeyCode::AltLeft => Some(0x3A),      // kVK_Option
        HidKeyCode::AltRight => Some(0x3D),     // kVK_RightOption
        HidKeyCode::MetaLeft => Some(0x37),     // kVK_Command
        HidKeyCode::MetaRight => Some(0x36),    // kVK_RightCommand

        HidKeyCode::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_letter_keys_have_macos_mappings() {
        for letter in [
            HidKeyCode::KeyA, HidKeyCode::KeyB, HidKeyCode::KeyC, HidKeyCode::KeyD,
            HidKeyCode::KeyE, HidKeyCode::KeyF, HidKeyCode::KeyG, HidKeyCode::KeyH,
            HidKeyCode::KeyI, HidKeyCode::KeyJ, HidKeyCode::KeyK, HidKeyCode::KeyL,
            HidKeyCode::KeyM, HidKeyCode::KeyN, HidKeyCode::KeyO, HidKeyCode::KeyP,
            HidKeyCode::KeyQ, HidKeyCode::KeyR, HidKeyCode::KeyS, HidKeyCode::KeyT,
            HidKeyCode::KeyU, HidKeyCode::KeyV, HidKeyCode::KeyW, HidKeyCode::KeyX,
            HidKeyCode::KeyY, HidKeyCode::KeyZ,
        ] {
            assert!(
                hid_to_cgkeycode(letter).is_some(),
                "{letter:?} should have a macOS CGKeyCode"
            );
        }
    }

    #[test]
    fn test_all_digit_keys_have_macos_mappings() {
        for digit in [
            HidKeyCode::Digit0, HidKeyCode::Digit1, HidKeyCode::Digit2,
            HidKeyCode::Digit3, HidKeyCode::Digit4, HidKeyCode::Digit5,
            HidKeyCode::Digit6, HidKeyCode::Digit7, HidKeyCode::Digit8,
            HidKeyCode::Digit9,
        ] {
            assert!(hid_to_cgkeycode(digit).is_some(), "{digit:?} should have a macOS CGKeyCode");
        }
    }

    #[test]
    fn test_all_function_keys_have_macos_mappings() {
        for fkey in [
            HidKeyCode::F1, HidKeyCode::F2, HidKeyCode::F3, HidKeyCode::F4,
            HidKeyCode::F5, HidKeyCode::F6, HidKeyCode::F7, HidKeyCode::F8,
            HidKeyCode::F9, HidKeyCode::F10, HidKeyCode::F11, HidKeyCode::F12,
        ] {
            assert!(hid_to_cgkeycode(fkey).is_some(), "{fkey:?} should have a macOS CGKeyCode");
        }
    }

    #[test]
    fn test_all_modifier_keys_have_macos_mappings() {
        for modifier in [
            HidKeyCode::ControlLeft, HidKeyCode::ControlRight,
            HidKeyCode::ShiftLeft, HidKeyCode::ShiftRight,
            HidKeyCode::AltLeft, HidKeyCode::AltRight,
            HidKeyCode::MetaLeft, HidKeyCode::MetaRight,
        ] {
            assert!(hid_to_cgkeycode(modifier).is_some(), "{modifier:?} should have a macOS CGKeyCode");
        }
    }

    #[test]
    fn test_unknown_hid_returns_none() {
        assert_eq!(hid_to_cgkeycode(HidKeyCode::Unknown), None);
    }

    #[test]
    fn test_key_a_maps_to_zero() {
        // kVK_ANSI_A = 0x00
        assert_eq!(hid_to_cgkeycode(HidKeyCode::KeyA), Some(0x00));
    }

    #[test]
    fn test_enter_maps_to_kVK_Return() {
        assert_eq!(hid_to_cgkeycode(HidKeyCode::Enter), Some(0x24));
    }

    #[test]
    fn test_arrow_keys_have_correct_cgkeycodes() {
        assert_eq!(hid_to_cgkeycode(HidKeyCode::ArrowLeft), Some(0x7B));
        assert_eq!(hid_to_cgkeycode(HidKeyCode::ArrowRight), Some(0x7C));
        assert_eq!(hid_to_cgkeycode(HidKeyCode::ArrowDown), Some(0x7D));
        assert_eq!(hid_to_cgkeycode(HidKeyCode::ArrowUp), Some(0x7E));
    }

    #[test]
    fn test_special_keys_have_macos_mappings() {
        for key in [
            HidKeyCode::Escape,
            HidKeyCode::Backspace,
            HidKeyCode::Tab,
            HidKeyCode::Space,
            HidKeyCode::CapsLock,
            HidKeyCode::Insert,
            HidKeyCode::Home,
            HidKeyCode::End,
            HidKeyCode::PageUp,
            HidKeyCode::PageDown,
            HidKeyCode::Delete,
            HidKeyCode::NumLock,
        ] {
            assert!(hid_to_cgkeycode(key).is_some(), "{key:?} should have a macOS CGKeyCode");
        }
    }
}
