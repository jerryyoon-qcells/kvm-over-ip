//! HID Usage ID to X11 KeySym translation table for Linux clients.
//!
//! X11 KeySym values are defined in X11/keysymdef.h.
//! Reference: https://gitlab.freedesktop.org/xorg/proto/xorgproto/-/blob/master/include/X11/keysymdef.h
//!
//! # What is an X11 KeySym? (for beginners)
//!
//! X11 is the windowing system used on Linux (and other Unix-like systems).
//! It uses a system called **KeySym** (Key Symbol) to identify keys.
//!
//! Unlike Windows VK codes (which identify physical key positions), X11 KeySyms
//! can represent *characters* as well as physical keys.  For example:
//!
//! | KeySym name | Value  | Meaning        |
//! |-------------|--------|----------------|
//! | `XK_a`      | 0x0061 | lowercase 'a'  |
//! | `XK_A`      | 0x0041 | uppercase 'A'  |
//! | `XK_Return` | 0xFF0D | Enter key      |
//! | `XK_Escape` | 0xFF1B | Escape key     |
//!
//! Notice that letters use their **ASCII values** (0x61 = 'a' in ASCII).
//! The XTest extension (`XTestFakeKeyEvent`) accepts a KeySym and synthesises
//! the appropriate key event on the X11 display.
//!
//! # Why lowercase letter KeySyms?
//!
//! This table maps letter keys to their *lowercase* KeySym (e.g., 0x0061 for 'a'
//! rather than 0x0041 for 'A').  The XTest extension translates automatically
//! when a Shift modifier is present, so we always pass the base (lowercase) form
//! and let X11 apply the appropriate modifier state.

use super::hid::HidKeyCode;

/// Translates a [`HidKeyCode`] to an X11 KeySym value.
///
/// Returns `None` if the HID code has no X11 KeySym equivalent.
///
/// # Panics
///
/// This function never panics.
pub fn hid_to_keysym(hid: HidKeyCode) -> Option<u32> {
    match hid {
        // Letters (X11 lowercase keysyms 0x61-0x7A)
        HidKeyCode::KeyA => Some(0x0061), // XK_a
        HidKeyCode::KeyB => Some(0x0062), // XK_b
        HidKeyCode::KeyC => Some(0x0063), // XK_c
        HidKeyCode::KeyD => Some(0x0064), // XK_d
        HidKeyCode::KeyE => Some(0x0065), // XK_e
        HidKeyCode::KeyF => Some(0x0066), // XK_f
        HidKeyCode::KeyG => Some(0x0067), // XK_g
        HidKeyCode::KeyH => Some(0x0068), // XK_h
        HidKeyCode::KeyI => Some(0x0069), // XK_i
        HidKeyCode::KeyJ => Some(0x006A), // XK_j
        HidKeyCode::KeyK => Some(0x006B), // XK_k
        HidKeyCode::KeyL => Some(0x006C), // XK_l
        HidKeyCode::KeyM => Some(0x006D), // XK_m
        HidKeyCode::KeyN => Some(0x006E), // XK_n
        HidKeyCode::KeyO => Some(0x006F), // XK_o
        HidKeyCode::KeyP => Some(0x0070), // XK_p
        HidKeyCode::KeyQ => Some(0x0071), // XK_q
        HidKeyCode::KeyR => Some(0x0072), // XK_r
        HidKeyCode::KeyS => Some(0x0073), // XK_s
        HidKeyCode::KeyT => Some(0x0074), // XK_t
        HidKeyCode::KeyU => Some(0x0075), // XK_u
        HidKeyCode::KeyV => Some(0x0076), // XK_v
        HidKeyCode::KeyW => Some(0x0077), // XK_w
        HidKeyCode::KeyX => Some(0x0078), // XK_x
        HidKeyCode::KeyY => Some(0x0079), // XK_y
        HidKeyCode::KeyZ => Some(0x007A), // XK_z

        // Digits (X11 0x30-0x39)
        HidKeyCode::Digit0 => Some(0x0030), // XK_0
        HidKeyCode::Digit1 => Some(0x0031), // XK_1
        HidKeyCode::Digit2 => Some(0x0032), // XK_2
        HidKeyCode::Digit3 => Some(0x0033), // XK_3
        HidKeyCode::Digit4 => Some(0x0034), // XK_4
        HidKeyCode::Digit5 => Some(0x0035), // XK_5
        HidKeyCode::Digit6 => Some(0x0036), // XK_6
        HidKeyCode::Digit7 => Some(0x0037), // XK_7
        HidKeyCode::Digit8 => Some(0x0038), // XK_8
        HidKeyCode::Digit9 => Some(0x0039), // XK_9

        // Control keys
        HidKeyCode::Enter => Some(0xFF0D),       // XK_Return
        HidKeyCode::Escape => Some(0xFF1B),      // XK_Escape
        HidKeyCode::Backspace => Some(0xFF08),   // XK_BackSpace
        HidKeyCode::Tab => Some(0xFF09),         // XK_Tab
        HidKeyCode::Space => Some(0x0020),       // XK_space
        HidKeyCode::CapsLock => Some(0xFFE5),    // XK_Caps_Lock
        HidKeyCode::ScrollLock => Some(0xFF14),  // XK_Scroll_Lock
        HidKeyCode::Pause => Some(0xFF13),       // XK_Pause
        HidKeyCode::Insert => Some(0xFF63),      // XK_Insert
        HidKeyCode::Home => Some(0xFF50),        // XK_Home
        HidKeyCode::PageUp => Some(0xFF55),      // XK_Page_Up
        HidKeyCode::Delete => Some(0xFFFF),      // XK_Delete
        HidKeyCode::End => Some(0xFF57),         // XK_End
        HidKeyCode::PageDown => Some(0xFF56),    // XK_Page_Down
        HidKeyCode::PrintScreen => Some(0xFF61), // XK_Print
        HidKeyCode::ContextMenu => Some(0xFF67), // XK_Menu

        // Arrow keys
        HidKeyCode::ArrowLeft => Some(0xFF51),  // XK_Left
        HidKeyCode::ArrowUp => Some(0xFF52),    // XK_Up
        HidKeyCode::ArrowRight => Some(0xFF53), // XK_Right
        HidKeyCode::ArrowDown => Some(0xFF54),  // XK_Down

        // Function keys
        HidKeyCode::F1 => Some(0xFFBE),  // XK_F1
        HidKeyCode::F2 => Some(0xFFBF),  // XK_F2
        HidKeyCode::F3 => Some(0xFFC0),  // XK_F3
        HidKeyCode::F4 => Some(0xFFC1),  // XK_F4
        HidKeyCode::F5 => Some(0xFFC2),  // XK_F5
        HidKeyCode::F6 => Some(0xFFC3),  // XK_F6
        HidKeyCode::F7 => Some(0xFFC4),  // XK_F7
        HidKeyCode::F8 => Some(0xFFC5),  // XK_F8
        HidKeyCode::F9 => Some(0xFFC6),  // XK_F9
        HidKeyCode::F10 => Some(0xFFC7), // XK_F10
        HidKeyCode::F11 => Some(0xFFC8), // XK_F11
        HidKeyCode::F12 => Some(0xFFC9), // XK_F12

        // Numpad
        HidKeyCode::NumLock => Some(0xFF7F),      // XK_Num_Lock
        HidKeyCode::NumpadDivide => Some(0xFFAF), // XK_KP_Divide
        HidKeyCode::NumpadMultiply => Some(0xFFAA), // XK_KP_Multiply
        HidKeyCode::NumpadSubtract => Some(0xFFAD), // XK_KP_Subtract
        HidKeyCode::NumpadAdd => Some(0xFFAB),    // XK_KP_Add
        HidKeyCode::NumpadEnter => Some(0xFF8D),  // XK_KP_Enter
        HidKeyCode::Numpad0 => Some(0xFFB0),      // XK_KP_0
        HidKeyCode::Numpad1 => Some(0xFFB1),      // XK_KP_1
        HidKeyCode::Numpad2 => Some(0xFFB2),      // XK_KP_2
        HidKeyCode::Numpad3 => Some(0xFFB3),      // XK_KP_3
        HidKeyCode::Numpad4 => Some(0xFFB4),      // XK_KP_4
        HidKeyCode::Numpad5 => Some(0xFFB5),      // XK_KP_5
        HidKeyCode::Numpad6 => Some(0xFFB6),      // XK_KP_6
        HidKeyCode::Numpad7 => Some(0xFFB7),      // XK_KP_7
        HidKeyCode::Numpad8 => Some(0xFFB8),      // XK_KP_8
        HidKeyCode::Numpad9 => Some(0xFFB9),      // XK_KP_9
        HidKeyCode::NumpadDecimal => Some(0xFFAE), // XK_KP_Decimal

        // Punctuation / symbols
        HidKeyCode::Minus => Some(0x002D),        // XK_minus
        HidKeyCode::Equal => Some(0x003D),        // XK_equal
        HidKeyCode::BracketLeft => Some(0x005B),  // XK_bracketleft
        HidKeyCode::BracketRight => Some(0x005D), // XK_bracketright
        HidKeyCode::Backslash => Some(0x005C),    // XK_backslash
        HidKeyCode::Semicolon => Some(0x003B),    // XK_semicolon
        HidKeyCode::Quote => Some(0x0027),        // XK_apostrophe
        HidKeyCode::Backquote => Some(0x0060),    // XK_grave
        HidKeyCode::Comma => Some(0x002C),        // XK_comma
        HidKeyCode::Period => Some(0x002E),       // XK_period
        HidKeyCode::Slash => Some(0x002F),        // XK_slash

        // Modifier keys
        HidKeyCode::ControlLeft => Some(0xFFE3), // XK_Control_L
        HidKeyCode::ControlRight => Some(0xFFE4), // XK_Control_R
        HidKeyCode::ShiftLeft => Some(0xFFE1),   // XK_Shift_L
        HidKeyCode::ShiftRight => Some(0xFFE2),  // XK_Shift_R
        HidKeyCode::AltLeft => Some(0xFFE9),     // XK_Alt_L
        HidKeyCode::AltRight => Some(0xFFEA),    // XK_Alt_R
        HidKeyCode::MetaLeft => Some(0xFFEB),    // XK_Super_L
        HidKeyCode::MetaRight => Some(0xFFEC),   // XK_Super_R

        HidKeyCode::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_letter_keys_have_x11_mappings() {
        let letters = [
            HidKeyCode::KeyA,
            HidKeyCode::KeyB,
            HidKeyCode::KeyC,
            HidKeyCode::KeyD,
            HidKeyCode::KeyE,
            HidKeyCode::KeyF,
            HidKeyCode::KeyG,
            HidKeyCode::KeyH,
            HidKeyCode::KeyI,
            HidKeyCode::KeyJ,
            HidKeyCode::KeyK,
            HidKeyCode::KeyL,
            HidKeyCode::KeyM,
            HidKeyCode::KeyN,
            HidKeyCode::KeyO,
            HidKeyCode::KeyP,
            HidKeyCode::KeyQ,
            HidKeyCode::KeyR,
            HidKeyCode::KeyS,
            HidKeyCode::KeyT,
            HidKeyCode::KeyU,
            HidKeyCode::KeyV,
            HidKeyCode::KeyW,
            HidKeyCode::KeyX,
            HidKeyCode::KeyY,
            HidKeyCode::KeyZ,
        ];
        for letter in letters {
            let result = hid_to_keysym(letter);
            assert!(result.is_some(), "{letter:?} should have an X11 keysym");
        }
    }

    #[test]
    fn test_all_digit_keys_have_x11_mappings() {
        for digit in [
            HidKeyCode::Digit0,
            HidKeyCode::Digit1,
            HidKeyCode::Digit2,
            HidKeyCode::Digit3,
            HidKeyCode::Digit4,
            HidKeyCode::Digit5,
            HidKeyCode::Digit6,
            HidKeyCode::Digit7,
            HidKeyCode::Digit8,
            HidKeyCode::Digit9,
        ] {
            assert!(
                hid_to_keysym(digit).is_some(),
                "{digit:?} should have an X11 keysym"
            );
        }
    }

    #[test]
    fn test_all_function_keys_have_x11_mappings() {
        for fkey in [
            HidKeyCode::F1,
            HidKeyCode::F2,
            HidKeyCode::F3,
            HidKeyCode::F4,
            HidKeyCode::F5,
            HidKeyCode::F6,
            HidKeyCode::F7,
            HidKeyCode::F8,
            HidKeyCode::F9,
            HidKeyCode::F10,
            HidKeyCode::F11,
            HidKeyCode::F12,
        ] {
            assert!(
                hid_to_keysym(fkey).is_some(),
                "{fkey:?} should have an X11 keysym"
            );
        }
    }

    #[test]
    fn test_all_modifier_keys_have_x11_mappings() {
        for modifier in [
            HidKeyCode::ControlLeft,
            HidKeyCode::ControlRight,
            HidKeyCode::ShiftLeft,
            HidKeyCode::ShiftRight,
            HidKeyCode::AltLeft,
            HidKeyCode::AltRight,
            HidKeyCode::MetaLeft,
            HidKeyCode::MetaRight,
        ] {
            assert!(
                hid_to_keysym(modifier).is_some(),
                "{modifier:?} should have an X11 keysym"
            );
        }
    }

    #[test]
    fn test_unknown_hid_returns_none() {
        assert_eq!(hid_to_keysym(HidKeyCode::Unknown), None);
    }

    #[test]
    fn test_enter_maps_to_xk_return() {
        assert_eq!(hid_to_keysym(HidKeyCode::Enter), Some(0xFF0D));
    }

    #[test]
    fn test_escape_maps_to_xk_escape() {
        assert_eq!(hid_to_keysym(HidKeyCode::Escape), Some(0xFF1B));
    }

    #[test]
    fn test_arrow_keys_have_correct_x11_keysyms() {
        assert_eq!(hid_to_keysym(HidKeyCode::ArrowLeft), Some(0xFF51));
        assert_eq!(hid_to_keysym(HidKeyCode::ArrowUp), Some(0xFF52));
        assert_eq!(hid_to_keysym(HidKeyCode::ArrowRight), Some(0xFF53));
        assert_eq!(hid_to_keysym(HidKeyCode::ArrowDown), Some(0xFF54));
    }

    #[test]
    fn test_letter_keysyms_are_lowercase_ascii() {
        // X11 keysyms for letters use lowercase ASCII values (0x61–0x7A)
        let pairs = [(HidKeyCode::KeyA, 0x0061u32), (HidKeyCode::KeyZ, 0x007A)];
        for (hid, expected) in pairs {
            assert_eq!(hid_to_keysym(hid), Some(expected));
        }
    }

    #[test]
    fn test_special_keys_have_x11_mappings() {
        for key in [
            HidKeyCode::PrintScreen,
            HidKeyCode::ScrollLock,
            HidKeyCode::Pause,
            HidKeyCode::Insert,
            HidKeyCode::Home,
            HidKeyCode::End,
            HidKeyCode::PageUp,
            HidKeyCode::PageDown,
            HidKeyCode::Delete,
            HidKeyCode::CapsLock,
            HidKeyCode::NumLock,
            HidKeyCode::ContextMenu,
        ] {
            assert!(
                hid_to_keysym(key).is_some(),
                "{key:?} should have an X11 keysym"
            );
        }
    }
}
