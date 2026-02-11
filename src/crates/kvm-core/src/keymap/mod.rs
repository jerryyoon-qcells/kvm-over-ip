//! Key code translation tables for cross-platform keyboard event mapping.
//!
//! The canonical representation is USB HID Usage IDs (page 0x07, Keyboard/Keypad).
//! Platform-specific codes are translated to/from HID at capture/emulation boundaries.

pub mod hid;
pub mod linux_x11;
pub mod macos_cg;
pub mod windows_vk;

pub use hid::HidKeyCode;

/// Unified key mapper providing all translation directions.
pub struct KeyMapper;

impl KeyMapper {
    /// Translates a Windows Virtual Key code to a [`HidKeyCode`].
    ///
    /// Returns [`HidKeyCode::Unknown`] if no mapping exists for `vk`.
    pub fn windows_vk_to_hid(vk: u8) -> HidKeyCode {
        windows_vk::vk_to_hid(vk)
    }

    /// Translates a [`HidKeyCode`] to a Windows Virtual Key code.
    ///
    /// Returns `None` if the HID code has no Windows VK equivalent.
    pub fn hid_to_windows_vk(hid: HidKeyCode) -> Option<u8> {
        windows_vk::hid_to_vk(hid)
    }

    /// Translates a [`HidKeyCode`] to an X11 KeySym value for Linux clients.
    ///
    /// Returns `None` if the HID code has no X11 equivalent.
    pub fn hid_to_x11_keysym(hid: HidKeyCode) -> Option<u32> {
        linux_x11::hid_to_keysym(hid)
    }

    /// Translates a [`HidKeyCode`] to a macOS `CGKeyCode` value.
    ///
    /// Returns `None` if the HID code has no macOS equivalent.
    pub fn hid_to_macos_cgkeycode(hid: HidKeyCode) -> Option<u16> {
        macos_cg::hid_to_cgkeycode(hid)
    }

    /// Translates a [`HidKeyCode`] to the DOM `KeyboardEvent.code` string for web clients.
    ///
    /// Returns `None` if the HID code has no DOM code equivalent.
    pub fn hid_to_dom_code(hid: HidKeyCode) -> Option<&'static str> {
        hid_to_dom_code_str(hid)
    }
}

/// DOM KeyboardEvent.code strings for web client input injection.
fn hid_to_dom_code_str(hid: HidKeyCode) -> Option<&'static str> {
    match hid {
        HidKeyCode::KeyA => Some("KeyA"),
        HidKeyCode::KeyB => Some("KeyB"),
        HidKeyCode::KeyC => Some("KeyC"),
        HidKeyCode::KeyD => Some("KeyD"),
        HidKeyCode::KeyE => Some("KeyE"),
        HidKeyCode::KeyF => Some("KeyF"),
        HidKeyCode::KeyG => Some("KeyG"),
        HidKeyCode::KeyH => Some("KeyH"),
        HidKeyCode::KeyI => Some("KeyI"),
        HidKeyCode::KeyJ => Some("KeyJ"),
        HidKeyCode::KeyK => Some("KeyK"),
        HidKeyCode::KeyL => Some("KeyL"),
        HidKeyCode::KeyM => Some("KeyM"),
        HidKeyCode::KeyN => Some("KeyN"),
        HidKeyCode::KeyO => Some("KeyO"),
        HidKeyCode::KeyP => Some("KeyP"),
        HidKeyCode::KeyQ => Some("KeyQ"),
        HidKeyCode::KeyR => Some("KeyR"),
        HidKeyCode::KeyS => Some("KeyS"),
        HidKeyCode::KeyT => Some("KeyT"),
        HidKeyCode::KeyU => Some("KeyU"),
        HidKeyCode::KeyV => Some("KeyV"),
        HidKeyCode::KeyW => Some("KeyW"),
        HidKeyCode::KeyX => Some("KeyX"),
        HidKeyCode::KeyY => Some("KeyY"),
        HidKeyCode::KeyZ => Some("KeyZ"),
        HidKeyCode::Digit1 => Some("Digit1"),
        HidKeyCode::Digit2 => Some("Digit2"),
        HidKeyCode::Digit3 => Some("Digit3"),
        HidKeyCode::Digit4 => Some("Digit4"),
        HidKeyCode::Digit5 => Some("Digit5"),
        HidKeyCode::Digit6 => Some("Digit6"),
        HidKeyCode::Digit7 => Some("Digit7"),
        HidKeyCode::Digit8 => Some("Digit8"),
        HidKeyCode::Digit9 => Some("Digit9"),
        HidKeyCode::Digit0 => Some("Digit0"),
        HidKeyCode::Enter => Some("Enter"),
        HidKeyCode::Escape => Some("Escape"),
        HidKeyCode::Backspace => Some("Backspace"),
        HidKeyCode::Tab => Some("Tab"),
        HidKeyCode::Space => Some("Space"),
        HidKeyCode::Minus => Some("Minus"),
        HidKeyCode::Equal => Some("Equal"),
        HidKeyCode::BracketLeft => Some("BracketLeft"),
        HidKeyCode::BracketRight => Some("BracketRight"),
        HidKeyCode::Backslash => Some("Backslash"),
        HidKeyCode::Semicolon => Some("Semicolon"),
        HidKeyCode::Quote => Some("Quote"),
        HidKeyCode::Backquote => Some("Backquote"),
        HidKeyCode::Comma => Some("Comma"),
        HidKeyCode::Period => Some("Period"),
        HidKeyCode::Slash => Some("Slash"),
        HidKeyCode::CapsLock => Some("CapsLock"),
        HidKeyCode::F1 => Some("F1"),
        HidKeyCode::F2 => Some("F2"),
        HidKeyCode::F3 => Some("F3"),
        HidKeyCode::F4 => Some("F4"),
        HidKeyCode::F5 => Some("F5"),
        HidKeyCode::F6 => Some("F6"),
        HidKeyCode::F7 => Some("F7"),
        HidKeyCode::F8 => Some("F8"),
        HidKeyCode::F9 => Some("F9"),
        HidKeyCode::F10 => Some("F10"),
        HidKeyCode::F11 => Some("F11"),
        HidKeyCode::F12 => Some("F12"),
        HidKeyCode::PrintScreen => Some("PrintScreen"),
        HidKeyCode::ScrollLock => Some("ScrollLock"),
        HidKeyCode::Pause => Some("Pause"),
        HidKeyCode::Insert => Some("Insert"),
        HidKeyCode::Home => Some("Home"),
        HidKeyCode::PageUp => Some("PageUp"),
        HidKeyCode::Delete => Some("Delete"),
        HidKeyCode::End => Some("End"),
        HidKeyCode::PageDown => Some("PageDown"),
        HidKeyCode::ArrowRight => Some("ArrowRight"),
        HidKeyCode::ArrowLeft => Some("ArrowLeft"),
        HidKeyCode::ArrowDown => Some("ArrowDown"),
        HidKeyCode::ArrowUp => Some("ArrowUp"),
        HidKeyCode::NumLock => Some("NumLock"),
        HidKeyCode::NumpadDivide => Some("NumpadDivide"),
        HidKeyCode::NumpadMultiply => Some("NumpadMultiply"),
        HidKeyCode::NumpadSubtract => Some("NumpadSubtract"),
        HidKeyCode::NumpadAdd => Some("NumpadAdd"),
        HidKeyCode::NumpadEnter => Some("NumpadEnter"),
        HidKeyCode::Numpad1 => Some("Numpad1"),
        HidKeyCode::Numpad2 => Some("Numpad2"),
        HidKeyCode::Numpad3 => Some("Numpad3"),
        HidKeyCode::Numpad4 => Some("Numpad4"),
        HidKeyCode::Numpad5 => Some("Numpad5"),
        HidKeyCode::Numpad6 => Some("Numpad6"),
        HidKeyCode::Numpad7 => Some("Numpad7"),
        HidKeyCode::Numpad8 => Some("Numpad8"),
        HidKeyCode::Numpad9 => Some("Numpad9"),
        HidKeyCode::Numpad0 => Some("Numpad0"),
        HidKeyCode::NumpadDecimal => Some("NumpadDecimal"),
        HidKeyCode::ContextMenu => Some("ContextMenu"),
        HidKeyCode::ControlLeft => Some("ControlLeft"),
        HidKeyCode::ShiftLeft => Some("ShiftLeft"),
        HidKeyCode::AltLeft => Some("AltLeft"),
        HidKeyCode::MetaLeft => Some("MetaLeft"),
        HidKeyCode::ControlRight => Some("ControlRight"),
        HidKeyCode::ShiftRight => Some("ShiftRight"),
        HidKeyCode::AltRight => Some("AltRight"),
        HidKeyCode::MetaRight => Some("MetaRight"),
        HidKeyCode::Unknown => None,
    }
}
