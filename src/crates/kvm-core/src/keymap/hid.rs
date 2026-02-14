//! USB HID Usage IDs (page 0x07, Keyboard/Keypad page).
//!
//! This is the canonical cross-platform key representation used throughout
//! the KVM-Over-IP protocol. All platform-specific codes are translated
//! to/from HID at the capture and emulation boundaries.
//!
//! Reference: USB HID Usage Tables 1.3, Section 10 (Keyboard/Keypad page 0x07).
//!
//! # What is a HID Usage ID? (for beginners)
//!
//! The **USB Human Interface Device (HID)** standard assigns a unique number to
//! every key on a keyboard.  These numbers are called *Usage IDs* and they are
//! grouped by *Usage Page*.  All keyboard keys are on page 0x07 ("Keyboard/Keypad").
//!
//! For example:
//!
//! | Key          | HID Usage ID |
//! |--------------|-------------|
//! | Letter A     | 0x04        |
//! | Letter B     | 0x05        |
//! | Enter        | 0x28        |
//! | Left Ctrl    | 0xE0        |
//!
//! Notice that HID codes for letters start at 0x04 (not at 'A'=0x41 like ASCII).
//! The reason is that HID codes represent **physical key positions**, not
//! characters.  The character that a key produces depends on the current keyboard
//! layout (QWERTY, AZERTY, Dvorak, etc.) and the modifier keys held down.
//! By using position codes, the system works correctly for all keyboard layouts.
//!
//! # The `Unknown` sentinel
//!
//! Not every key has a universally assigned HID code (e.g., some multimedia
//! keys are vendor-specific).  [`HidKeyCode::Unknown`] (value 0x0000) is used
//! as a placeholder for any key that has no standard mapping.  The encoder
//! will still transmit the event with code 0x0000 so the receiver can decide
//! what to do with it.

use serde::{Deserialize, Serialize};

/// USB HID Usage ID for keyboard keys (page 0x07).
///
/// The numeric value of each variant is its HID Usage ID on the keyboard/keypad page.
/// [`HidKeyCode::Unknown`] represents any key that has no mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum HidKeyCode {
    // Letters (HID 0x04–0x1D)
    KeyA = 0x04,
    KeyB = 0x05,
    KeyC = 0x06,
    KeyD = 0x07,
    KeyE = 0x08,
    KeyF = 0x09,
    KeyG = 0x0A,
    KeyH = 0x0B,
    KeyI = 0x0C,
    KeyJ = 0x0D,
    KeyK = 0x0E,
    KeyL = 0x0F,
    KeyM = 0x10,
    KeyN = 0x11,
    KeyO = 0x12,
    KeyP = 0x13,
    KeyQ = 0x14,
    KeyR = 0x15,
    KeyS = 0x16,
    KeyT = 0x17,
    KeyU = 0x18,
    KeyV = 0x19,
    KeyW = 0x1A,
    KeyX = 0x1B,
    KeyY = 0x1C,
    KeyZ = 0x1D,

    // Digits (HID 0x1E–0x27)
    Digit1 = 0x1E,
    Digit2 = 0x1F,
    Digit3 = 0x20,
    Digit4 = 0x21,
    Digit5 = 0x22,
    Digit6 = 0x23,
    Digit7 = 0x24,
    Digit8 = 0x25,
    Digit9 = 0x26,
    Digit0 = 0x27,

    // Control keys (HID 0x28–0x38)
    Enter = 0x28,
    Escape = 0x29,
    Backspace = 0x2A,
    Tab = 0x2B,
    Space = 0x2C,
    Minus = 0x2D,
    Equal = 0x2E,
    BracketLeft = 0x2F,
    BracketRight = 0x30,
    Backslash = 0x31,
    Semicolon = 0x33,
    Quote = 0x34,
    Backquote = 0x35,
    Comma = 0x36,
    Period = 0x37,
    Slash = 0x38,

    // Lock keys
    CapsLock = 0x39,

    // Function keys (HID 0x3A–0x45)
    F1 = 0x3A,
    F2 = 0x3B,
    F3 = 0x3C,
    F4 = 0x3D,
    F5 = 0x3E,
    F6 = 0x3F,
    F7 = 0x40,
    F8 = 0x41,
    F9 = 0x42,
    F10 = 0x43,
    F11 = 0x44,
    F12 = 0x45,

    // Navigation cluster (HID 0x46–0x52)
    PrintScreen = 0x46,
    ScrollLock = 0x47,
    Pause = 0x48,
    Insert = 0x49,
    Home = 0x4A,
    PageUp = 0x4B,
    Delete = 0x4C,
    End = 0x4D,
    PageDown = 0x4E,
    ArrowRight = 0x4F,
    ArrowLeft = 0x50,
    ArrowDown = 0x51,
    ArrowUp = 0x52,

    // Numpad (HID 0x53–0x63)
    NumLock = 0x53,
    NumpadDivide = 0x54,
    NumpadMultiply = 0x55,
    NumpadSubtract = 0x56,
    NumpadAdd = 0x57,
    NumpadEnter = 0x58,
    Numpad1 = 0x59,
    Numpad2 = 0x5A,
    Numpad3 = 0x5B,
    Numpad4 = 0x5C,
    Numpad5 = 0x5D,
    Numpad6 = 0x5E,
    Numpad7 = 0x5F,
    Numpad8 = 0x60,
    Numpad9 = 0x61,
    Numpad0 = 0x62,
    NumpadDecimal = 0x63,

    // Application key (HID 0x65)
    ContextMenu = 0x65,

    // Modifier keys (HID 0xE0–0xE7)
    ControlLeft = 0xE0,
    ShiftLeft = 0xE1,
    AltLeft = 0xE2,
    MetaLeft = 0xE3,
    ControlRight = 0xE4,
    ShiftRight = 0xE5,
    AltRight = 0xE6,
    MetaRight = 0xE7,

    /// Sentinel for keys with no HID mapping.
    Unknown = 0x0000,
}

impl HidKeyCode {
    /// Converts a raw u16 HID Usage ID to a [`HidKeyCode`].
    ///
    /// Returns [`HidKeyCode::Unknown`] if the value does not correspond to a
    /// known key code variant.
    pub fn from_u16(value: u16) -> Self {
        match value {
            0x04 => HidKeyCode::KeyA,
            0x05 => HidKeyCode::KeyB,
            0x06 => HidKeyCode::KeyC,
            0x07 => HidKeyCode::KeyD,
            0x08 => HidKeyCode::KeyE,
            0x09 => HidKeyCode::KeyF,
            0x0A => HidKeyCode::KeyG,
            0x0B => HidKeyCode::KeyH,
            0x0C => HidKeyCode::KeyI,
            0x0D => HidKeyCode::KeyJ,
            0x0E => HidKeyCode::KeyK,
            0x0F => HidKeyCode::KeyL,
            0x10 => HidKeyCode::KeyM,
            0x11 => HidKeyCode::KeyN,
            0x12 => HidKeyCode::KeyO,
            0x13 => HidKeyCode::KeyP,
            0x14 => HidKeyCode::KeyQ,
            0x15 => HidKeyCode::KeyR,
            0x16 => HidKeyCode::KeyS,
            0x17 => HidKeyCode::KeyT,
            0x18 => HidKeyCode::KeyU,
            0x19 => HidKeyCode::KeyV,
            0x1A => HidKeyCode::KeyW,
            0x1B => HidKeyCode::KeyX,
            0x1C => HidKeyCode::KeyY,
            0x1D => HidKeyCode::KeyZ,
            0x1E => HidKeyCode::Digit1,
            0x1F => HidKeyCode::Digit2,
            0x20 => HidKeyCode::Digit3,
            0x21 => HidKeyCode::Digit4,
            0x22 => HidKeyCode::Digit5,
            0x23 => HidKeyCode::Digit6,
            0x24 => HidKeyCode::Digit7,
            0x25 => HidKeyCode::Digit8,
            0x26 => HidKeyCode::Digit9,
            0x27 => HidKeyCode::Digit0,
            0x28 => HidKeyCode::Enter,
            0x29 => HidKeyCode::Escape,
            0x2A => HidKeyCode::Backspace,
            0x2B => HidKeyCode::Tab,
            0x2C => HidKeyCode::Space,
            0x2D => HidKeyCode::Minus,
            0x2E => HidKeyCode::Equal,
            0x2F => HidKeyCode::BracketLeft,
            0x30 => HidKeyCode::BracketRight,
            0x31 => HidKeyCode::Backslash,
            0x33 => HidKeyCode::Semicolon,
            0x34 => HidKeyCode::Quote,
            0x35 => HidKeyCode::Backquote,
            0x36 => HidKeyCode::Comma,
            0x37 => HidKeyCode::Period,
            0x38 => HidKeyCode::Slash,
            0x39 => HidKeyCode::CapsLock,
            0x3A => HidKeyCode::F1,
            0x3B => HidKeyCode::F2,
            0x3C => HidKeyCode::F3,
            0x3D => HidKeyCode::F4,
            0x3E => HidKeyCode::F5,
            0x3F => HidKeyCode::F6,
            0x40 => HidKeyCode::F7,
            0x41 => HidKeyCode::F8,
            0x42 => HidKeyCode::F9,
            0x43 => HidKeyCode::F10,
            0x44 => HidKeyCode::F11,
            0x45 => HidKeyCode::F12,
            0x46 => HidKeyCode::PrintScreen,
            0x47 => HidKeyCode::ScrollLock,
            0x48 => HidKeyCode::Pause,
            0x49 => HidKeyCode::Insert,
            0x4A => HidKeyCode::Home,
            0x4B => HidKeyCode::PageUp,
            0x4C => HidKeyCode::Delete,
            0x4D => HidKeyCode::End,
            0x4E => HidKeyCode::PageDown,
            0x4F => HidKeyCode::ArrowRight,
            0x50 => HidKeyCode::ArrowLeft,
            0x51 => HidKeyCode::ArrowDown,
            0x52 => HidKeyCode::ArrowUp,
            0x53 => HidKeyCode::NumLock,
            0x54 => HidKeyCode::NumpadDivide,
            0x55 => HidKeyCode::NumpadMultiply,
            0x56 => HidKeyCode::NumpadSubtract,
            0x57 => HidKeyCode::NumpadAdd,
            0x58 => HidKeyCode::NumpadEnter,
            0x59 => HidKeyCode::Numpad1,
            0x5A => HidKeyCode::Numpad2,
            0x5B => HidKeyCode::Numpad3,
            0x5C => HidKeyCode::Numpad4,
            0x5D => HidKeyCode::Numpad5,
            0x5E => HidKeyCode::Numpad6,
            0x5F => HidKeyCode::Numpad7,
            0x60 => HidKeyCode::Numpad8,
            0x61 => HidKeyCode::Numpad9,
            0x62 => HidKeyCode::Numpad0,
            0x63 => HidKeyCode::NumpadDecimal,
            0x65 => HidKeyCode::ContextMenu,
            0xE0 => HidKeyCode::ControlLeft,
            0xE1 => HidKeyCode::ShiftLeft,
            0xE2 => HidKeyCode::AltLeft,
            0xE3 => HidKeyCode::MetaLeft,
            0xE4 => HidKeyCode::ControlRight,
            0xE5 => HidKeyCode::ShiftRight,
            0xE6 => HidKeyCode::AltRight,
            0xE7 => HidKeyCode::MetaRight,
            _ => HidKeyCode::Unknown,
        }
    }

    /// Returns the raw USB HID Usage ID value for this key code.
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// Returns `true` if this is a modifier key.
    pub fn is_modifier(self) -> bool {
        matches!(
            self,
            HidKeyCode::ControlLeft
                | HidKeyCode::ControlRight
                | HidKeyCode::ShiftLeft
                | HidKeyCode::ShiftRight
                | HidKeyCode::AltLeft
                | HidKeyCode::AltRight
                | HidKeyCode::MetaLeft
                | HidKeyCode::MetaRight
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All standard HID key codes that must have valid from_u16/as_u16 round-trips.
    const STANDARD_KEYS: &[(u16, HidKeyCode)] = &[
        (0x04, HidKeyCode::KeyA),
        (0x05, HidKeyCode::KeyB),
        (0x1E, HidKeyCode::Digit1),
        (0x27, HidKeyCode::Digit0),
        (0x28, HidKeyCode::Enter),
        (0x29, HidKeyCode::Escape),
        (0x2A, HidKeyCode::Backspace),
        (0x2B, HidKeyCode::Tab),
        (0x2C, HidKeyCode::Space),
        (0x39, HidKeyCode::CapsLock),
        (0x3A, HidKeyCode::F1),
        (0x45, HidKeyCode::F12),
        (0x46, HidKeyCode::PrintScreen),
        (0x47, HidKeyCode::ScrollLock),
        (0x48, HidKeyCode::Pause),
        (0x49, HidKeyCode::Insert),
        (0x4A, HidKeyCode::Home),
        (0x4B, HidKeyCode::PageUp),
        (0x4C, HidKeyCode::Delete),
        (0x4D, HidKeyCode::End),
        (0x4E, HidKeyCode::PageDown),
        (0x4F, HidKeyCode::ArrowRight),
        (0x50, HidKeyCode::ArrowLeft),
        (0x51, HidKeyCode::ArrowDown),
        (0x52, HidKeyCode::ArrowUp),
        (0x53, HidKeyCode::NumLock),
        (0x58, HidKeyCode::NumpadEnter),
        (0x62, HidKeyCode::Numpad0),
        (0x65, HidKeyCode::ContextMenu),
        (0xE0, HidKeyCode::ControlLeft),
        (0xE1, HidKeyCode::ShiftLeft),
        (0xE2, HidKeyCode::AltLeft),
        (0xE3, HidKeyCode::MetaLeft),
        (0xE4, HidKeyCode::ControlRight),
        (0xE5, HidKeyCode::ShiftRight),
        (0xE6, HidKeyCode::AltRight),
        (0xE7, HidKeyCode::MetaRight),
    ];

    #[test]
    fn test_from_u16_produces_correct_key_codes_for_all_standard_keys() {
        for &(raw, expected) in STANDARD_KEYS {
            // Arrange / Act
            let result = HidKeyCode::from_u16(raw);

            // Assert
            assert_eq!(
                result, expected,
                "from_u16(0x{raw:04X}) should produce {expected:?}"
            );
        }
    }

    #[test]
    fn test_as_u16_returns_correct_hid_value_for_all_standard_keys() {
        for &(expected_raw, code) in STANDARD_KEYS {
            // Arrange / Act
            let raw = code.as_u16();

            // Assert
            assert_eq!(
                raw, expected_raw,
                "{code:?}.as_u16() should return 0x{expected_raw:04X}"
            );
        }
    }

    #[test]
    fn test_round_trip_from_u16_and_as_u16() {
        for &(raw, _) in STANDARD_KEYS {
            // Arrange / Act
            let code = HidKeyCode::from_u16(raw);
            let back = code.as_u16();

            // Assert
            assert_eq!(raw, back, "round-trip for 0x{raw:04X} failed");
        }
    }

    #[test]
    fn test_unknown_u16_values_return_unknown() {
        // Values that are not assigned in HID keyboard/keypad page
        for unassigned in [0x00, 0x01, 0x02, 0x03, 0x32, 0x64, 0xA0, 0xFF] {
            let result = HidKeyCode::from_u16(unassigned);
            assert_eq!(
                result,
                HidKeyCode::Unknown,
                "0x{unassigned:02X} should map to Unknown"
            );
        }
    }

    #[test]
    fn test_unknown_code_returns_zero_from_as_u16() {
        assert_eq!(HidKeyCode::Unknown.as_u16(), 0x0000);
    }

    #[test]
    fn test_modifier_keys_are_identified_correctly() {
        let modifiers = [
            HidKeyCode::ControlLeft,
            HidKeyCode::ControlRight,
            HidKeyCode::ShiftLeft,
            HidKeyCode::ShiftRight,
            HidKeyCode::AltLeft,
            HidKeyCode::AltRight,
            HidKeyCode::MetaLeft,
            HidKeyCode::MetaRight,
        ];
        for m in modifiers {
            assert!(m.is_modifier(), "{m:?} should be a modifier key");
        }
    }

    #[test]
    fn test_non_modifier_keys_are_not_identified_as_modifiers() {
        let non_modifiers = [
            HidKeyCode::KeyA,
            HidKeyCode::Enter,
            HidKeyCode::Escape,
            HidKeyCode::F1,
            HidKeyCode::Space,
            HidKeyCode::Numpad0,
            HidKeyCode::Unknown,
        ];
        for k in non_modifiers {
            assert!(!k.is_modifier(), "{k:?} should NOT be a modifier key");
        }
    }

    #[test]
    fn test_all_letter_keys_covered() {
        // Verify all 26 letters have valid HID codes in expected range 0x04-0x1D
        let letters = [
            HidKeyCode::KeyA, HidKeyCode::KeyB, HidKeyCode::KeyC, HidKeyCode::KeyD,
            HidKeyCode::KeyE, HidKeyCode::KeyF, HidKeyCode::KeyG, HidKeyCode::KeyH,
            HidKeyCode::KeyI, HidKeyCode::KeyJ, HidKeyCode::KeyK, HidKeyCode::KeyL,
            HidKeyCode::KeyM, HidKeyCode::KeyN, HidKeyCode::KeyO, HidKeyCode::KeyP,
            HidKeyCode::KeyQ, HidKeyCode::KeyR, HidKeyCode::KeyS, HidKeyCode::KeyT,
            HidKeyCode::KeyU, HidKeyCode::KeyV, HidKeyCode::KeyW, HidKeyCode::KeyX,
            HidKeyCode::KeyY, HidKeyCode::KeyZ,
        ];
        assert_eq!(letters.len(), 26, "should have exactly 26 letter keys");
        for (i, &letter) in letters.iter().enumerate() {
            let expected_hid = 0x04u16 + i as u16;
            assert_eq!(
                letter.as_u16(),
                expected_hid,
                "{letter:?} should have HID code 0x{expected_hid:04X}"
            );
        }
    }
}
