//! Windows Virtual Key (VK) code to USB HID Usage ID translation table.
//!
//! Reference: Windows Virtual-Key Codes (winuser.h) and USB HID Usage Tables 1.3.
//! Windows VK codes range from 0x00 to 0xFF.
//!
//! # What is a Windows Virtual Key (VK) code? (for beginners)
//!
//! Windows assigns each keyboard key a number called a "Virtual Key code".
//! These are defined in `<winuser.h>` and named `VK_*` (e.g., `VK_RETURN = 0x0D`,
//! `VK_SPACE = 0x20`).  They are "virtual" because they represent *logical* keys
//! rather than physical scan codes: pressing the letter A on any keyboard layout
//! always produces `VK_A = 0x41`, regardless of whether the physical key is
//! labelled "A" (QWERTY) or "Q" (AZERTY).
//!
//! # How this table works
//!
//! `VK_TO_HID_TABLE` is a compile-time constant array of 256 [`HidKeyCode`]
//! values, indexed by VK code.  Position 0x41 holds `HidKeyCode::KeyA` because
//! Windows VK_A is 0x41.  Any VK code without a HID equivalent stores
//! `HidKeyCode::Unknown`.
//!
//! Indexing into this array is an O(1) lookup — the fastest possible operation.
//! This is important because every captured key event goes through this table.

use super::hid::HidKeyCode;

/// Translates a Windows Virtual Key code to a HID Usage ID.
///
/// Returns [`HidKeyCode::Unknown`] for VK codes that have no keyboard HID equivalent
/// (e.g., mouse button VKs, browser keys without HID mappings).
///
/// # Panics
///
/// This function never panics; all u8 inputs are handled.
pub fn vk_to_hid(vk: u8) -> HidKeyCode {
    VK_TO_HID_TABLE[vk as usize]
}

/// Translates a HID Usage ID back to a Windows Virtual Key code.
///
/// Returns `None` for HID codes with no VK equivalent.
pub fn hid_to_vk(hid: HidKeyCode) -> Option<u8> {
    // Linear scan is acceptable for the infrequent HID->VK direction.
    // The table is 256 entries, so worst-case is 256 comparisons.
    for (vk, &mapped_hid) in VK_TO_HID_TABLE.iter().enumerate() {
        if mapped_hid == hid && hid != HidKeyCode::Unknown {
            return Some(vk as u8);
        }
    }
    None
}

/// Complete VK → HID mapping table indexed by VK code (0x00–0xFF).
///
/// Entries are `HidKeyCode::Unknown` when no keyboard HID equivalent exists.
/// Reference: https://learn.microsoft.com/windows/win32/inputdev/virtual-key-codes
const VK_TO_HID_TABLE: [HidKeyCode; 256] = {
    use HidKeyCode::*;
    let mut t = [Unknown; 256];

    // ── Alphabet keys (VK_A=0x41 … VK_Z=0x5A) ────────────────────────────────
    t[0x41] = KeyA;
    t[0x42] = KeyB;
    t[0x43] = KeyC;
    t[0x44] = KeyD;
    t[0x45] = KeyE;
    t[0x46] = KeyF;
    t[0x47] = KeyG;
    t[0x48] = KeyH;
    t[0x49] = KeyI;
    t[0x4A] = KeyJ;
    t[0x4B] = KeyK;
    t[0x4C] = KeyL;
    t[0x4D] = KeyM;
    t[0x4E] = KeyN;
    t[0x4F] = KeyO;
    t[0x50] = KeyP;
    t[0x51] = KeyQ;
    t[0x52] = KeyR;
    t[0x53] = KeyS;
    t[0x54] = KeyT;
    t[0x55] = KeyU;
    t[0x56] = KeyV;
    t[0x57] = KeyW;
    t[0x58] = KeyX;
    t[0x59] = KeyY;
    t[0x5A] = KeyZ;

    // ── Digit row (VK_0=0x30 … VK_9=0x39) ───────────────────────────────────
    t[0x30] = Digit0;
    t[0x31] = Digit1;
    t[0x32] = Digit2;
    t[0x33] = Digit3;
    t[0x34] = Digit4;
    t[0x35] = Digit5;
    t[0x36] = Digit6;
    t[0x37] = Digit7;
    t[0x38] = Digit8;
    t[0x39] = Digit9;

    // ── Control keys ─────────────────────────────────────────────────────────
    t[0x0D] = Enter;        // VK_RETURN
    t[0x1B] = Escape;       // VK_ESCAPE
    t[0x08] = Backspace;    // VK_BACK
    t[0x09] = Tab;          // VK_TAB
    t[0x20] = Space;        // VK_SPACE
    t[0x14] = CapsLock;     // VK_CAPITAL
    t[0x91] = ScrollLock;   // VK_SCROLL
    t[0x13] = Pause;        // VK_PAUSE
    t[0x2D] = Insert;       // VK_INSERT
    t[0x24] = Home;         // VK_HOME
    t[0x21] = PageUp;       // VK_PRIOR
    t[0x2E] = Delete;       // VK_DELETE
    t[0x23] = End;          // VK_END
    t[0x22] = PageDown;     // VK_NEXT
    t[0x2C] = PrintScreen;  // VK_SNAPSHOT
    t[0x5D] = ContextMenu;  // VK_APPS

    // ── Arrow keys ────────────────────────────────────────────────────────────
    t[0x25] = ArrowLeft;
    t[0x26] = ArrowUp;
    t[0x27] = ArrowRight;
    t[0x28] = ArrowDown;

    // ── Function keys (VK_F1=0x70 … VK_F12=0x7B) ─────────────────────────────
    t[0x70] = F1;
    t[0x71] = F2;
    t[0x72] = F3;
    t[0x73] = F4;
    t[0x74] = F5;
    t[0x75] = F6;
    t[0x76] = F7;
    t[0x77] = F8;
    t[0x78] = F9;
    t[0x79] = F10;
    t[0x7A] = F11;
    t[0x7B] = F12;

    // ── Numpad (VK_NUMPAD0=0x60 … VK_NUMPAD9=0x69) ───────────────────────────
    t[0x60] = Numpad0;
    t[0x61] = Numpad1;
    t[0x62] = Numpad2;
    t[0x63] = Numpad3;
    t[0x64] = Numpad4;
    t[0x65] = Numpad5;
    t[0x66] = Numpad6;
    t[0x67] = Numpad7;
    t[0x68] = Numpad8;
    t[0x69] = Numpad9;
    t[0x6A] = NumpadMultiply;   // VK_MULTIPLY
    t[0x6B] = NumpadAdd;        // VK_ADD
    t[0x6D] = NumpadSubtract;   // VK_SUBTRACT
    t[0x6E] = NumpadDecimal;    // VK_DECIMAL
    t[0x6F] = NumpadDivide;     // VK_DIVIDE
    t[0x90] = NumLock;          // VK_NUMLOCK

    // ── Punctuation / symbols ─────────────────────────────────────────────────
    t[0xBD] = Minus;        // VK_OEM_MINUS  (- _)
    t[0xBB] = Equal;        // VK_OEM_PLUS   (= +)
    t[0xDB] = BracketLeft;  // VK_OEM_4      ([ {)
    t[0xDD] = BracketRight; // VK_OEM_6      (] })
    t[0xDC] = Backslash;    // VK_OEM_5      (\ |)
    t[0xBA] = Semicolon;    // VK_OEM_1      (; :)
    t[0xDE] = Quote;        // VK_OEM_7      (' ")
    t[0xC0] = Backquote;    // VK_OEM_3      (` ~)
    t[0xBC] = Comma;        // VK_OEM_COMMA  (, <)
    t[0xBE] = Period;       // VK_OEM_PERIOD (. >)
    t[0xBF] = Slash;        // VK_OEM_2      (/ ?)

    // ── Modifier keys ─────────────────────────────────────────────────────────
    t[0xA2] = ControlLeft;  // VK_LCONTROL
    t[0xA3] = ControlRight; // VK_RCONTROL
    t[0xA0] = ShiftLeft;    // VK_LSHIFT
    t[0xA1] = ShiftRight;   // VK_RSHIFT
    t[0xA4] = AltLeft;      // VK_LMENU
    t[0xA5] = AltRight;     // VK_RMENU
    t[0x5B] = MetaLeft;     // VK_LWIN
    t[0x5C] = MetaRight;    // VK_RWIN

    // ── Numpad Enter (extended) ───────────────────────────────────────────────
    // Note: WH_KEYBOARD_LL delivers VK_RETURN with extended flag for numpad Enter.
    // The infrastructure layer must detect this and use NumpadEnter.
    // For the table, VK_RETURN maps to Enter (non-extended path).

    t
};

#[cfg(test)]
mod tests {
    use super::*;
    use HidKeyCode::*;

    /// Pairs of (VK code, expected HID code) for all standard US QWERTY keys.
    const STANDARD_MAPPINGS: &[(u8, HidKeyCode)] = &[
        // Letters
        (0x41, KeyA), (0x42, KeyB), (0x43, KeyC), (0x44, KeyD), (0x45, KeyE),
        (0x46, KeyF), (0x47, KeyG), (0x48, KeyH), (0x49, KeyI), (0x4A, KeyJ),
        (0x4B, KeyK), (0x4C, KeyL), (0x4D, KeyM), (0x4E, KeyN), (0x4F, KeyO),
        (0x50, KeyP), (0x51, KeyQ), (0x52, KeyR), (0x53, KeyS), (0x54, KeyT),
        (0x55, KeyU), (0x56, KeyV), (0x57, KeyW), (0x58, KeyX), (0x59, KeyY),
        (0x5A, KeyZ),
        // Digits
        (0x30, Digit0), (0x31, Digit1), (0x32, Digit2), (0x33, Digit3), (0x34, Digit4),
        (0x35, Digit5), (0x36, Digit6), (0x37, Digit7), (0x38, Digit8), (0x39, Digit9),
        // Function keys
        (0x70, F1), (0x71, F2), (0x72, F3), (0x73, F4), (0x74, F5), (0x75, F6),
        (0x76, F7), (0x77, F8), (0x78, F9), (0x79, F10), (0x7A, F11), (0x7B, F12),
        // Navigation
        (0x25, ArrowLeft), (0x26, ArrowUp), (0x27, ArrowRight), (0x28, ArrowDown),
        (0x24, Home), (0x23, End), (0x21, PageUp), (0x22, PageDown),
        (0x2D, Insert), (0x2E, Delete),
        // Control keys
        (0x0D, Enter), (0x1B, Escape), (0x08, Backspace), (0x09, Tab), (0x20, Space),
        (0x14, CapsLock), (0x91, ScrollLock), (0x13, Pause), (0x2C, PrintScreen),
        // Numpad
        (0x60, Numpad0), (0x61, Numpad1), (0x62, Numpad2), (0x63, Numpad3),
        (0x64, Numpad4), (0x65, Numpad5), (0x66, Numpad6), (0x67, Numpad7),
        (0x68, Numpad8), (0x69, Numpad9),
        (0x6A, NumpadMultiply), (0x6B, NumpadAdd), (0x6D, NumpadSubtract),
        (0x6E, NumpadDecimal), (0x6F, NumpadDivide), (0x90, NumLock),
        // Modifiers
        (0xA2, ControlLeft), (0xA3, ControlRight),
        (0xA0, ShiftLeft), (0xA1, ShiftRight),
        (0xA4, AltLeft), (0xA5, AltRight),
        (0x5B, MetaLeft), (0x5C, MetaRight),
        // Punctuation
        (0xBD, Minus), (0xBB, Equal), (0xDB, BracketLeft), (0xDD, BracketRight),
        (0xDC, Backslash), (0xBA, Semicolon), (0xDE, Quote), (0xC0, Backquote),
        (0xBC, Comma), (0xBE, Period), (0xBF, Slash),
        (0x5D, ContextMenu),
    ];

    #[test]
    fn test_all_standard_vk_codes_map_to_correct_hid() {
        for &(vk, expected_hid) in STANDARD_MAPPINGS {
            let result = vk_to_hid(vk);
            assert_eq!(
                result, expected_hid,
                "vk_to_hid(0x{vk:02X}) should return {expected_hid:?}"
            );
        }
    }

    #[test]
    fn test_all_hid_codes_map_back_to_vk_bidirectionally() {
        for &(expected_vk, hid) in STANDARD_MAPPINGS {
            let result = hid_to_vk(hid);
            assert_eq!(
                result,
                Some(expected_vk),
                "hid_to_vk({hid:?}) should return Some(0x{expected_vk:02X})"
            );
        }
    }

    #[test]
    fn test_unknown_vk_codes_return_unknown_hid() {
        // VK codes that have no keyboard HID equivalent
        for vk in [0x00u8, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x0A] {
            let result = vk_to_hid(vk);
            assert_eq!(
                result,
                HidKeyCode::Unknown,
                "vk_to_hid(0x{vk:02X}) should be Unknown (mouse/undefined VK)"
            );
        }
    }

    #[test]
    fn test_hid_to_vk_unknown_returns_none() {
        assert_eq!(hid_to_vk(HidKeyCode::Unknown), None);
    }

    #[test]
    fn test_vk_to_hid_never_panics_for_any_u8() {
        // This test verifies the table is complete for all 256 possible u8 inputs.
        for vk in 0u8..=255 {
            let _ = vk_to_hid(vk); // Must not panic
        }
    }

    #[test]
    fn test_all_26_letter_keys_are_mapped() {
        let letter_vks: Vec<u8> = (0x41u8..=0x5Au8).collect();
        assert_eq!(letter_vks.len(), 26);
        for vk in letter_vks {
            let hid = vk_to_hid(vk);
            assert_ne!(hid, HidKeyCode::Unknown, "VK 0x{vk:02X} must have a HID mapping");
        }
    }

    #[test]
    fn test_round_trip_vk_to_hid_to_vk_for_all_standard_keys() {
        for &(vk, _) in STANDARD_MAPPINGS {
            let hid = vk_to_hid(vk);
            let back = hid_to_vk(hid);
            assert_eq!(
                back,
                Some(vk),
                "round-trip failed for VK 0x{vk:02X}: got {back:?}"
            );
        }
    }
}
