//! EmulateInputUseCase: translates received protocol messages to OS input events.
//!
//! This use case sits at the application layer and delegates to a
//! [`PlatformInputEmulator`] trait object for OS-level event injection.
//! The platform-specific implementations are in the infrastructure layer.

use kvm_core::{
    keymap::hid::HidKeyCode,
    protocol::messages::{
        ButtonEventType, KeyEventMessage, KeyEventType, MouseButton, MouseButtonMessage,
        ModifierFlags, MouseMoveMessage, MouseScrollMessage,
    },
};
use thiserror::Error;

/// Error type for input emulation operations.
#[derive(Debug, Error)]
pub enum EmulationError {
    #[error("platform error: {0}")]
    Platform(String),
    #[error("invalid key code: {0:?}")]
    InvalidKeyCode(HidKeyCode),
    #[error("emulator not initialized")]
    NotInitialized,
}

/// Platform-agnostic input emulation trait.
///
/// Each supported OS provides an implementation in the infrastructure layer.
pub trait PlatformInputEmulator: Send + Sync {
    /// Emulates a key press (key-down event).
    fn emit_key_down(
        &self,
        key: HidKeyCode,
        modifiers: ModifierFlags,
    ) -> Result<(), EmulationError>;

    /// Emulates a key release (key-up event).
    fn emit_key_up(
        &self,
        key: HidKeyCode,
        modifiers: ModifierFlags,
    ) -> Result<(), EmulationError>;

    /// Moves the cursor to an absolute position in the client's coordinate space.
    fn emit_mouse_move(&self, x: i32, y: i32) -> Result<(), EmulationError>;

    /// Emulates a mouse button press or release.
    fn emit_mouse_button(
        &self,
        button: MouseButton,
        pressed: bool,
        x: i32,
        y: i32,
    ) -> Result<(), EmulationError>;

    /// Emulates mouse wheel scroll.
    fn emit_mouse_scroll(&self, delta_x: i16, delta_y: i16) -> Result<(), EmulationError>;
}

/// Filters duplicate consecutive events to avoid injecting the same event twice.
#[derive(Default)]
struct DedupFilter {
    last_mouse_pos: Option<(i32, i32)>,
}

impl DedupFilter {
    fn should_send_mouse_move(&mut self, x: i32, y: i32) -> bool {
        if self.last_mouse_pos == Some((x, y)) {
            return false;
        }
        self.last_mouse_pos = Some((x, y));
        true
    }

    fn reset(&mut self) {
        self.last_mouse_pos = None;
    }
}

/// The Emulate Input use case.
///
/// Receives decoded protocol messages and dispatches them to the platform emulator.
pub struct EmulateInputUseCase {
    emulator: std::sync::Arc<dyn PlatformInputEmulator>,
    dedup: DedupFilter,
}

impl EmulateInputUseCase {
    /// Creates a new use case with the given platform emulator.
    pub fn new(emulator: std::sync::Arc<dyn PlatformInputEmulator>) -> Self {
        Self {
            emulator,
            dedup: DedupFilter::default(),
        }
    }

    /// Handles a key event from the master.
    ///
    /// # Errors
    ///
    /// Returns [`EmulationError`] if the OS event injection fails.
    pub fn handle_key_event(&self, event: &KeyEventMessage) -> Result<(), EmulationError> {
        match event.event_type {
            KeyEventType::KeyDown => self.emulator.emit_key_down(event.key_code, event.modifiers),
            KeyEventType::KeyUp => self.emulator.emit_key_up(event.key_code, event.modifiers),
        }
    }

    /// Handles a mouse move event from the master.
    ///
    /// Duplicate consecutive positions are filtered out.
    ///
    /// # Errors
    ///
    /// Returns [`EmulationError`] if the OS event injection fails.
    pub fn handle_mouse_move(&mut self, event: &MouseMoveMessage) -> Result<(), EmulationError> {
        if self.dedup.should_send_mouse_move(event.x, event.y) {
            self.emulator.emit_mouse_move(event.x, event.y)?;
        }
        Ok(())
    }

    /// Handles a mouse button event from the master.
    ///
    /// # Errors
    ///
    /// Returns [`EmulationError`] if the OS event injection fails.
    pub fn handle_mouse_button(&self, event: &MouseButtonMessage) -> Result<(), EmulationError> {
        let pressed = matches!(event.event_type, ButtonEventType::Press);
        self.emulator
            .emit_mouse_button(event.button, pressed, event.x, event.y)
    }

    /// Handles a mouse scroll event from the master.
    ///
    /// # Errors
    ///
    /// Returns [`EmulationError`] if the OS event injection fails.
    pub fn handle_mouse_scroll(&self, event: &MouseScrollMessage) -> Result<(), EmulationError> {
        self.emulator.emit_mouse_scroll(event.delta_x, event.delta_y)
    }

    /// Resets internal state (e.g., on reconnect).
    pub fn reset(&mut self) {
        self.dedup.reset();
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kvm_core::protocol::messages::{MouseButton, ButtonEventType};
    use std::sync::{Arc, Mutex};

    // ── Mock emulator ─────────────────────────────────────────────────────────

    #[derive(Default)]
    struct RecordingEmulator {
        key_downs: Mutex<Vec<HidKeyCode>>,
        key_ups: Mutex<Vec<HidKeyCode>>,
        mouse_moves: Mutex<Vec<(i32, i32)>>,
        mouse_buttons: Mutex<Vec<(MouseButton, bool)>>,
        scrolls: Mutex<Vec<(i16, i16)>>,
        should_fail: bool,
    }

    impl PlatformInputEmulator for RecordingEmulator {
        fn emit_key_down(&self, key: HidKeyCode, _: ModifierFlags) -> Result<(), EmulationError> {
            if self.should_fail {
                return Err(EmulationError::Platform("injected failure".to_string()));
            }
            self.key_downs.lock().unwrap().push(key);
            Ok(())
        }

        fn emit_key_up(&self, key: HidKeyCode, _: ModifierFlags) -> Result<(), EmulationError> {
            if self.should_fail {
                return Err(EmulationError::Platform("injected failure".to_string()));
            }
            self.key_ups.lock().unwrap().push(key);
            Ok(())
        }

        fn emit_mouse_move(&self, x: i32, y: i32) -> Result<(), EmulationError> {
            if self.should_fail {
                return Err(EmulationError::Platform("injected failure".to_string()));
            }
            self.mouse_moves.lock().unwrap().push((x, y));
            Ok(())
        }

        fn emit_mouse_button(
            &self,
            button: MouseButton,
            pressed: bool,
            _x: i32,
            _y: i32,
        ) -> Result<(), EmulationError> {
            if self.should_fail {
                return Err(EmulationError::Platform("injected failure".to_string()));
            }
            self.mouse_buttons.lock().unwrap().push((button, pressed));
            Ok(())
        }

        fn emit_mouse_scroll(&self, delta_x: i16, delta_y: i16) -> Result<(), EmulationError> {
            if self.should_fail {
                return Err(EmulationError::Platform("injected failure".to_string()));
            }
            self.scrolls.lock().unwrap().push((delta_x, delta_y));
            Ok(())
        }
    }

    fn make_use_case() -> (EmulateInputUseCase, Arc<RecordingEmulator>) {
        let emulator = Arc::new(RecordingEmulator::default());
        let uc = EmulateInputUseCase::new(Arc::clone(&emulator) as Arc<dyn PlatformInputEmulator>);
        (uc, emulator)
    }

    // ── Key events ────────────────────────────────────────────────────────────

    #[test]
    fn test_handle_key_down_calls_emit_key_down() {
        // Arrange
        let (uc, em) = make_use_case();
        let event = KeyEventMessage {
            key_code: HidKeyCode::KeyA,
            scan_code: 0x1E,
            event_type: KeyEventType::KeyDown,
            modifiers: ModifierFlags::default(),
        };

        // Act
        uc.handle_key_event(&event).unwrap();

        // Assert
        assert_eq!(*em.key_downs.lock().unwrap(), vec![HidKeyCode::KeyA]);
        assert!(em.key_ups.lock().unwrap().is_empty());
    }

    #[test]
    fn test_handle_key_up_calls_emit_key_up() {
        // Arrange
        let (uc, em) = make_use_case();
        let event = KeyEventMessage {
            key_code: HidKeyCode::Enter,
            scan_code: 0x1C,
            event_type: KeyEventType::KeyUp,
            modifiers: ModifierFlags::default(),
        };

        // Act
        uc.handle_key_event(&event).unwrap();

        // Assert
        assert_eq!(*em.key_ups.lock().unwrap(), vec![HidKeyCode::Enter]);
        assert!(em.key_downs.lock().unwrap().is_empty());
    }

    // ── Mouse move ────────────────────────────────────────────────────────────

    #[test]
    fn test_handle_mouse_move_sends_position_to_emulator() {
        // Arrange
        let (mut uc, em) = make_use_case();
        let event = MouseMoveMessage { x: 640, y: 480, delta_x: 0, delta_y: 0 };

        // Act
        uc.handle_mouse_move(&event).unwrap();

        // Assert
        assert_eq!(*em.mouse_moves.lock().unwrap(), vec![(640, 480)]);
    }

    #[test]
    fn test_handle_mouse_move_deduplicates_identical_consecutive_positions() {
        // Arrange
        let (mut uc, em) = make_use_case();
        let event = MouseMoveMessage { x: 100, y: 200, delta_x: 0, delta_y: 0 };

        // Act – send same position twice
        uc.handle_mouse_move(&event).unwrap();
        uc.handle_mouse_move(&event).unwrap();

        // Assert – emulator receives only one call
        assert_eq!(em.mouse_moves.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_handle_mouse_move_does_not_deduplicate_different_positions() {
        // Arrange
        let (mut uc, em) = make_use_case();

        // Act
        uc.handle_mouse_move(&MouseMoveMessage { x: 100, y: 200, delta_x: 0, delta_y: 0 }).unwrap();
        uc.handle_mouse_move(&MouseMoveMessage { x: 101, y: 200, delta_x: 1, delta_y: 0 }).unwrap();

        // Assert – both positions sent
        assert_eq!(em.mouse_moves.lock().unwrap().len(), 2);
    }

    #[test]
    fn test_reset_clears_dedup_state() {
        // Arrange
        let (mut uc, em) = make_use_case();
        let event = MouseMoveMessage { x: 100, y: 200, delta_x: 0, delta_y: 0 };
        uc.handle_mouse_move(&event).unwrap(); // first send

        // Act
        uc.reset();
        uc.handle_mouse_move(&event).unwrap(); // should send again after reset

        // Assert
        assert_eq!(em.mouse_moves.lock().unwrap().len(), 2);
    }

    // ── Mouse buttons ─────────────────────────────────────────────────────────

    #[test]
    fn test_handle_mouse_button_press_calls_emit_with_pressed_true() {
        // Arrange
        let (uc, em) = make_use_case();
        let event = MouseButtonMessage {
            button: MouseButton::Left,
            event_type: ButtonEventType::Press,
            x: 500,
            y: 400,
        };

        // Act
        uc.handle_mouse_button(&event).unwrap();

        // Assert
        let buttons = em.mouse_buttons.lock().unwrap();
        assert_eq!(buttons.len(), 1);
        assert_eq!(buttons[0], (MouseButton::Left, true));
    }

    #[test]
    fn test_handle_mouse_button_release_calls_emit_with_pressed_false() {
        // Arrange
        let (uc, em) = make_use_case();
        let event = MouseButtonMessage {
            button: MouseButton::Right,
            event_type: ButtonEventType::Release,
            x: 0,
            y: 0,
        };

        // Act
        uc.handle_mouse_button(&event).unwrap();

        // Assert
        let buttons = em.mouse_buttons.lock().unwrap();
        assert_eq!(buttons[0], (MouseButton::Right, false));
    }

    // ── Scroll ────────────────────────────────────────────────────────────────

    #[test]
    fn test_handle_mouse_scroll_vertical() {
        // Arrange
        let (uc, em) = make_use_case();
        let event = MouseScrollMessage { delta_x: 0, delta_y: 120, x: 0, y: 0 };

        // Act
        uc.handle_mouse_scroll(&event).unwrap();

        // Assert
        assert_eq!(*em.scrolls.lock().unwrap(), vec![(0, 120)]);
    }

    #[test]
    fn test_handle_mouse_scroll_horizontal() {
        // Arrange
        let (uc, em) = make_use_case();
        let event = MouseScrollMessage { delta_x: -120, delta_y: 0, x: 0, y: 0 };

        // Act
        uc.handle_mouse_scroll(&event).unwrap();

        // Assert
        assert_eq!(*em.scrolls.lock().unwrap(), vec![(-120, 0)]);
    }
}
