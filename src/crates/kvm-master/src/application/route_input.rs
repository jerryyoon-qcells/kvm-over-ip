//! RouteInputUseCase: routes captured input events to the correct destination.
//!
//! This use case is the heart of the master application. It receives raw input
//! events from the capture service, consults the [`VirtualLayout`] for routing
//! decisions, and dispatches translated messages to the [`InputTransmitter`].
//!
//! # Architecture
//!
//! This use case depends only on traits (`InputTransmitter`, `CursorController`)
//! and domain types (`VirtualLayout`). All infrastructure implementations are
//! injected at construction time, making the use case fully unit-testable.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use kvm_core::{
    domain::layout::{CursorLocation, EdgeTransition, ScreenId, VirtualLayout},
    keymap::{hid::HidKeyCode, KeyMapper},
    protocol::messages::{
        KeyEventMessage, KeyEventType, ModifierFlags, MouseButton as ProtoMouseButton,
        MouseButtonMessage, MouseMoveMessage, MouseScrollMessage, ButtonEventType,
    },
    ClientId,
};
use thiserror::Error;
use uuid::Uuid;

use crate::infrastructure::input_capture::{MouseButton as RawMouseButton, RawInputEvent};

/// Debounce duration for edge transitions to prevent oscillation.
const TRANSITION_DEBOUNCE: Duration = Duration::from_millis(50);

/// Error type for the route-input use case.
#[derive(Debug, Error)]
pub enum RouteError {
    #[error("transmitter error: {0}")]
    Transmit(String),
    #[error("no layout configured")]
    NoLayout,
}

/// Trait for sending translated input events to a remote client.
///
/// Infrastructure implementations use DTLS; test implementations record calls.
#[async_trait]
pub trait InputTransmitter: Send + Sync {
    /// Sends a keyboard event to the specified client.
    async fn send_key_event(
        &self,
        client_id: ClientId,
        event: KeyEventMessage,
    ) -> Result<(), String>;

    /// Sends a mouse move event to the specified client.
    async fn send_mouse_move(
        &self,
        client_id: ClientId,
        event: MouseMoveMessage,
    ) -> Result<(), String>;

    /// Sends a mouse button event to the specified client.
    async fn send_mouse_button(
        &self,
        client_id: ClientId,
        event: MouseButtonMessage,
    ) -> Result<(), String>;

    /// Sends a mouse scroll event to the specified client.
    async fn send_mouse_scroll(
        &self,
        client_id: ClientId,
        event: MouseScrollMessage,
    ) -> Result<(), String>;
}

/// Trait for controlling the physical master cursor position.
///
/// Infrastructure implementation calls `SetCursorPos`; test implementation records calls.
pub trait CursorController: Send + Sync {
    /// Teleports the physical cursor to (x, y) in master-local coordinates.
    fn teleport_cursor(&self, x: i32, y: i32);

    /// Returns the current physical cursor position in master-local coordinates.
    fn get_cursor_pos(&self) -> (i32, i32);
}

/// Tracks which machine currently has the active keyboard/mouse focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveTarget {
    /// Input goes to the local master system.
    Master,
    /// Input is routed to the specified client.
    Client(ClientId),
}

impl Default for ActiveTarget {
    fn default() -> Self {
        ActiveTarget::Master
    }
}

/// The current modifier key state maintained across key-down/up events.
#[derive(Debug, Default, Clone, Copy)]
struct ModifierState {
    left_ctrl: bool,
    right_ctrl: bool,
    left_shift: bool,
    right_shift: bool,
    left_alt: bool,
    right_alt: bool,
    left_meta: bool,
    right_meta: bool,
}

impl ModifierState {
    fn to_flags(self) -> ModifierFlags {
        let mut flags = 0u8;
        if self.left_ctrl { flags |= ModifierFlags::LEFT_CTRL; }
        if self.right_ctrl { flags |= ModifierFlags::RIGHT_CTRL; }
        if self.left_shift { flags |= ModifierFlags::LEFT_SHIFT; }
        if self.right_shift { flags |= ModifierFlags::RIGHT_SHIFT; }
        if self.left_alt { flags |= ModifierFlags::LEFT_ALT; }
        if self.right_alt { flags |= ModifierFlags::RIGHT_ALT; }
        if self.left_meta { flags |= ModifierFlags::LEFT_META; }
        if self.right_meta { flags |= ModifierFlags::RIGHT_META; }
        ModifierFlags(flags)
    }

    fn update(&mut self, vk: u8, is_down: bool) {
        match vk {
            0xA2 => self.left_ctrl = is_down,
            0xA3 => self.right_ctrl = is_down,
            0xA0 => self.left_shift = is_down,
            0xA1 => self.right_shift = is_down,
            0xA4 => self.left_alt = is_down,
            0xA5 => self.right_alt = is_down,
            0x5B => self.left_meta = is_down,
            0x5C => self.right_meta = is_down,
            _ => {}
        }
    }
}

/// The Route Input use case.
///
/// Receives raw captured events, translates them to protocol messages, and
/// routes them to the correct destination (local or remote client).
pub struct RouteInputUseCase {
    layout: VirtualLayout,
    active_target: ActiveTarget,
    cursor_pos: (i32, i32),
    sharing_enabled: bool,
    hotkey_vk: u8,
    modifiers: ModifierState,
    last_transition: Option<Instant>,
    transmitter: Arc<dyn InputTransmitter>,
    cursor_controller: Arc<dyn CursorController>,
    /// Sequence counter for outbound messages.
    sequence: u64,
}

impl RouteInputUseCase {
    /// Creates a new use case instance.
    pub fn new(
        master_width: u32,
        master_height: u32,
        transmitter: Arc<dyn InputTransmitter>,
        cursor_controller: Arc<dyn CursorController>,
        hotkey_vk: u8,
    ) -> Self {
        Self {
            layout: VirtualLayout::new(master_width, master_height),
            active_target: ActiveTarget::Master,
            cursor_pos: (0, 0),
            sharing_enabled: true,
            hotkey_vk,
            modifiers: ModifierState::default(),
            last_transition: None,
            transmitter,
            cursor_controller,
            sequence: 0,
        }
    }

    /// Replaces the layout with an updated configuration.
    ///
    /// If the active client is no longer in the new layout, routing falls back to master.
    pub fn update_layout(&mut self, layout: VirtualLayout) {
        // If the active client was removed, fall back to master
        if let ActiveTarget::Client(cid) = &self.active_target {
            let still_exists = layout.clients().any(|c| c.client_id == *cid);
            if !still_exists {
                self.active_target = ActiveTarget::Master;
            }
        }
        self.layout = layout;
    }

    /// Returns the currently active routing target.
    pub fn get_active_target(&self) -> &ActiveTarget {
        &self.active_target
    }

    /// Returns whether sharing is currently enabled.
    pub fn is_sharing_enabled(&self) -> bool {
        self.sharing_enabled
    }

    /// Enables or disables input sharing.
    pub fn set_sharing_enabled(&mut self, enabled: bool) {
        self.sharing_enabled = enabled;
        if !enabled {
            self.active_target = ActiveTarget::Master;
        }
    }

    /// Handles a raw input event from the capture service.
    ///
    /// Returns the current active target after handling.
    ///
    /// # Errors
    ///
    /// Returns [`RouteError::Transmit`] if the transmitter fails to deliver the event.
    pub async fn handle_event(&mut self, event: RawInputEvent) -> Result<(), RouteError> {
        match event {
            RawInputEvent::KeyDown {
                vk_code,
                scan_code,
                ..
            } => {
                self.modifiers.update(vk_code, true);
                self.handle_key_down(vk_code, scan_code).await?;
            }
            RawInputEvent::KeyUp {
                vk_code,
                scan_code,
                ..
            } => {
                self.modifiers.update(vk_code, false);
                self.handle_key_up(vk_code, scan_code).await?;
            }
            RawInputEvent::MouseMove { x, y, .. } => {
                self.handle_mouse_move(x, y).await?;
            }
            RawInputEvent::MouseButtonDown { button, x, y, .. } => {
                self.handle_mouse_button(button, true, x, y).await?;
            }
            RawInputEvent::MouseButtonUp { button, x, y, .. } => {
                self.handle_mouse_button(button, false, x, y).await?;
            }
            RawInputEvent::MouseWheel { delta, x, y, .. } => {
                self.handle_mouse_scroll(0, delta, x, y).await?;
            }
            RawInputEvent::MouseWheelH { delta, x, y, .. } => {
                self.handle_mouse_scroll(delta, 0, x, y).await?;
            }
        }
        Ok(())
    }

    // ── Private event handlers ────────────────────────────────────────────────

    async fn handle_key_down(&mut self, vk_code: u8, scan_code: u16) -> Result<(), RouteError> {
        // Check for hotkey (disable/enable sharing)
        if vk_code == self.hotkey_vk {
            self.sharing_enabled = !self.sharing_enabled;
            if !self.sharing_enabled {
                self.active_target = ActiveTarget::Master;
            }
            return Ok(());
        }

        if !self.sharing_enabled {
            return Ok(());
        }

        if let ActiveTarget::Client(cid) = self.active_target.clone() {
            let hid = KeyMapper::windows_vk_to_hid(vk_code);
            if hid == HidKeyCode::Unknown {
                return Ok(());
            }
            let event = KeyEventMessage {
                key_code: hid,
                scan_code,
                event_type: KeyEventType::KeyDown,
                modifiers: self.modifiers.to_flags(),
            };
            self.transmitter
                .send_key_event(cid, event)
                .await
                .map_err(RouteError::Transmit)?;
        }
        Ok(())
    }

    async fn handle_key_up(&mut self, vk_code: u8, scan_code: u16) -> Result<(), RouteError> {
        if !self.sharing_enabled {
            return Ok(());
        }
        if let ActiveTarget::Client(cid) = self.active_target.clone() {
            let hid = KeyMapper::windows_vk_to_hid(vk_code);
            if hid == HidKeyCode::Unknown {
                return Ok(());
            }
            let event = KeyEventMessage {
                key_code: hid,
                scan_code,
                event_type: KeyEventType::KeyUp,
                modifiers: self.modifiers.to_flags(),
            };
            self.transmitter
                .send_key_event(cid, event)
                .await
                .map_err(RouteError::Transmit)?;
        }
        Ok(())
    }

    async fn handle_mouse_move(&mut self, x: i32, y: i32) -> Result<(), RouteError> {
        self.cursor_pos = (x, y);

        if !self.sharing_enabled {
            return Ok(());
        }

        let current_screen = match &self.active_target {
            ActiveTarget::Master => ScreenId::Master,
            ActiveTarget::Client(cid) => ScreenId::Client(*cid),
        };

        // Compute local position for edge detection
        let (local_x, local_y) = match &current_screen {
            ScreenId::Master => (x, y),
            ScreenId::Client(cid) => {
                if let Some(client) = self.layout.clients().find(|c| c.client_id == *cid) {
                    (x - client.region.virtual_x, y - client.region.virtual_y)
                } else {
                    // Client disappeared from layout; fall back to master
                    self.active_target = ActiveTarget::Master;
                    return Ok(());
                }
            }
        };

        // Check for edge transition (with debounce)
        let can_transition = self
            .last_transition
            .map(|t| t.elapsed() >= TRANSITION_DEBOUNCE)
            .unwrap_or(true);

        if can_transition {
            if let Some(transition) = self.layout.check_edge_transition(&current_screen, local_x, local_y) {
                return self.apply_transition(transition).await;
            }
        }

        // No transition – send mouse move to active client if any
        if let ActiveTarget::Client(cid) = self.active_target.clone() {
            let event = MouseMoveMessage {
                x: local_x,
                y: local_y,
                delta_x: 0, // delta computed by client from sequential positions
                delta_y: 0,
            };
            self.transmitter
                .send_mouse_move(cid, event)
                .await
                .map_err(RouteError::Transmit)?;
        }
        Ok(())
    }

    async fn apply_transition(&mut self, transition: EdgeTransition) -> Result<(), RouteError> {
        self.last_transition = Some(Instant::now());

        // Update the active target
        self.active_target = match &transition.to_screen {
            ScreenId::Master => ActiveTarget::Master,
            ScreenId::Client(cid) => ActiveTarget::Client(*cid),
        };

        // Teleport the physical cursor to prevent it from straying off the master screen
        self.cursor_controller.teleport_cursor(
            transition.master_teleport_x,
            transition.master_teleport_y,
        );

        // Send the entry position to the new target if it's a client
        if let ActiveTarget::Client(cid) = self.active_target.clone() {
            let event = MouseMoveMessage {
                x: transition.entry_x,
                y: transition.entry_y,
                delta_x: 0,
                delta_y: 0,
            };
            self.transmitter
                .send_mouse_move(cid, event)
                .await
                .map_err(RouteError::Transmit)?;
        }
        Ok(())
    }

    async fn handle_mouse_button(
        &mut self,
        button: RawMouseButton,
        pressed: bool,
        x: i32,
        y: i32,
    ) -> Result<(), RouteError> {
        if !self.sharing_enabled {
            return Ok(());
        }
        if let ActiveTarget::Client(cid) = self.active_target.clone() {
            let proto_button = match button {
                RawMouseButton::Left => ProtoMouseButton::Left,
                RawMouseButton::Right => ProtoMouseButton::Right,
                RawMouseButton::Middle => ProtoMouseButton::Middle,
                RawMouseButton::X1 => ProtoMouseButton::Button4,
                RawMouseButton::X2 => ProtoMouseButton::Button5,
            };
            let event_type = if pressed {
                ButtonEventType::Press
            } else {
                ButtonEventType::Release
            };
            let event = MouseButtonMessage {
                button: proto_button,
                event_type,
                x,
                y,
            };
            self.transmitter
                .send_mouse_button(cid, event)
                .await
                .map_err(RouteError::Transmit)?;
        }
        Ok(())
    }

    async fn handle_mouse_scroll(
        &mut self,
        delta_x: i16,
        delta_y: i16,
        x: i32,
        y: i32,
    ) -> Result<(), RouteError> {
        if !self.sharing_enabled {
            return Ok(());
        }
        if let ActiveTarget::Client(cid) = self.active_target.clone() {
            let event = MouseScrollMessage { delta_x, delta_y, x, y };
            self.transmitter
                .send_mouse_scroll(cid, event)
                .await
                .map_err(RouteError::Transmit)?;
        }
        Ok(())
    }

    fn next_sequence(&mut self) -> u64 {
        let seq = self.sequence;
        self.sequence = self.sequence.wrapping_add(1);
        seq
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kvm_core::domain::layout::{Adjacency, ClientScreen, Edge, ScreenId, ScreenRegion};
    use std::sync::Mutex;
    use uuid::Uuid;

    // ── Test doubles ──────────────────────────────────────────────────────────

    #[derive(Default)]
    struct RecordingTransmitter {
        key_events: Mutex<Vec<(ClientId, KeyEventMessage)>>,
        mouse_moves: Mutex<Vec<(ClientId, MouseMoveMessage)>>,
        mouse_buttons: Mutex<Vec<(ClientId, MouseButtonMessage)>>,
        mouse_scrolls: Mutex<Vec<(ClientId, MouseScrollMessage)>>,
        should_fail: bool,
    }

    #[async_trait]
    impl InputTransmitter for RecordingTransmitter {
        async fn send_key_event(
            &self,
            client_id: ClientId,
            event: KeyEventMessage,
        ) -> Result<(), String> {
            if self.should_fail {
                return Err("injected failure".to_string());
            }
            self.key_events.lock().unwrap().push((client_id, event));
            Ok(())
        }

        async fn send_mouse_move(
            &self,
            client_id: ClientId,
            event: MouseMoveMessage,
        ) -> Result<(), String> {
            if self.should_fail {
                return Err("injected failure".to_string());
            }
            self.mouse_moves.lock().unwrap().push((client_id, event));
            Ok(())
        }

        async fn send_mouse_button(
            &self,
            client_id: ClientId,
            event: MouseButtonMessage,
        ) -> Result<(), String> {
            if self.should_fail {
                return Err("injected failure".to_string());
            }
            self.mouse_buttons.lock().unwrap().push((client_id, event));
            Ok(())
        }

        async fn send_mouse_scroll(
            &self,
            client_id: ClientId,
            event: MouseScrollMessage,
        ) -> Result<(), String> {
            if self.should_fail {
                return Err("injected failure".to_string());
            }
            self.mouse_scrolls.lock().unwrap().push((client_id, event));
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingCursorController {
        teleport_calls: Mutex<Vec<(i32, i32)>>,
        cursor_pos: Mutex<(i32, i32)>,
    }

    impl CursorController for RecordingCursorController {
        fn teleport_cursor(&self, x: i32, y: i32) {
            *self.cursor_pos.lock().unwrap() = (x, y);
            self.teleport_calls.lock().unwrap().push((x, y));
        }

        fn get_cursor_pos(&self) -> (i32, i32) {
            *self.cursor_pos.lock().unwrap()
        }
    }

    fn make_use_case_with_client(
        cid: Uuid,
    ) -> (
        RouteInputUseCase,
        Arc<RecordingTransmitter>,
        Arc<RecordingCursorController>,
    ) {
        let transmitter = Arc::new(RecordingTransmitter::default());
        let cursor = Arc::new(RecordingCursorController::default());
        let mut uc = RouteInputUseCase::new(
            1920,
            1080,
            Arc::clone(&transmitter) as Arc<dyn InputTransmitter>,
            Arc::clone(&cursor) as Arc<dyn CursorController>,
            0x91, // ScrollLock as hotkey VK
        );

        // Add a client to the right
        uc.layout
            .add_client(ClientScreen {
                client_id: cid,
                region: ScreenRegion {
                    virtual_x: 1920,
                    virtual_y: 0,
                    width: 1920,
                    height: 1080,
                },
                name: "test-client".to_string(),
            })
            .unwrap();
        uc.layout
            .set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Right,
                to_screen: ScreenId::Client(cid),
                to_edge: Edge::Left,
            })
            .unwrap();

        (uc, transmitter, cursor)
    }

    // ── Keyboard routing ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_key_event_routes_to_active_client() {
        // Arrange
        let cid = Uuid::new_v4();
        let (mut uc, tx, _cursor) = make_use_case_with_client(cid);
        uc.active_target = ActiveTarget::Client(cid);

        // Act
        uc.handle_event(RawInputEvent::KeyDown {
            vk_code: 0x41, // VK_A
            scan_code: 0x1E,
            time_ms: 0,
            is_extended: false,
        })
        .await
        .unwrap();

        // Assert
        let events = tx.key_events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, cid);
        assert_eq!(events[0].1.key_code, HidKeyCode::KeyA);
        assert_eq!(events[0].1.event_type, KeyEventType::KeyDown);
    }

    #[tokio::test]
    async fn test_key_event_not_routed_when_active_target_is_master() {
        // Arrange
        let cid = Uuid::new_v4();
        let (mut uc, tx, _) = make_use_case_with_client(cid);
        uc.active_target = ActiveTarget::Master; // default

        // Act
        uc.handle_event(RawInputEvent::KeyDown {
            vk_code: 0x41,
            scan_code: 0x1E,
            time_ms: 0,
            is_extended: false,
        })
        .await
        .unwrap();

        // Assert
        assert!(tx.key_events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_hotkey_toggles_sharing_off() {
        // Arrange
        let cid = Uuid::new_v4();
        let (mut uc, _, _) = make_use_case_with_client(cid);
        uc.active_target = ActiveTarget::Client(cid);
        assert!(uc.is_sharing_enabled());

        // Act – press ScrollLock (0x91)
        uc.handle_event(RawInputEvent::KeyDown {
            vk_code: 0x91,
            scan_code: 0,
            time_ms: 0,
            is_extended: false,
        })
        .await
        .unwrap();

        // Assert
        assert!(!uc.is_sharing_enabled());
        assert_eq!(uc.get_active_target(), &ActiveTarget::Master);
    }

    #[tokio::test]
    async fn test_hotkey_toggles_sharing_back_on() {
        // Arrange
        let cid = Uuid::new_v4();
        let (mut uc, _, _) = make_use_case_with_client(cid);
        uc.set_sharing_enabled(false);

        // Act
        uc.handle_event(RawInputEvent::KeyDown {
            vk_code: 0x91,
            scan_code: 0,
            time_ms: 0,
            is_extended: false,
        })
        .await
        .unwrap();

        // Assert
        assert!(uc.is_sharing_enabled());
    }

    #[tokio::test]
    async fn test_key_event_not_routed_when_sharing_disabled() {
        // Arrange
        let cid = Uuid::new_v4();
        let (mut uc, tx, _) = make_use_case_with_client(cid);
        uc.active_target = ActiveTarget::Client(cid);
        uc.set_sharing_enabled(false);

        // Act
        uc.handle_event(RawInputEvent::KeyDown {
            vk_code: 0x41,
            scan_code: 0x1E,
            time_ms: 0,
            is_extended: false,
        })
        .await
        .unwrap();

        // Assert – nothing transmitted when sharing is disabled
        assert!(tx.key_events.lock().unwrap().is_empty());
    }

    // ── Mouse routing ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_mouse_move_routed_to_active_client() {
        // Arrange
        let cid = Uuid::new_v4();
        let (mut uc, tx, _) = make_use_case_with_client(cid);
        uc.active_target = ActiveTarget::Client(cid);
        // Set layout to know client region start at virtual_x=1920
        uc.cursor_pos = (2000, 100);

        // Act – move within client area
        uc.handle_event(RawInputEvent::MouseMove {
            x: 2010,
            y: 200,
            time_ms: 0,
        })
        .await
        .unwrap();

        // Assert
        let moves = tx.mouse_moves.lock().unwrap();
        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].0, cid);
    }

    #[tokio::test]
    async fn test_mouse_move_not_routed_when_on_master() {
        // Arrange
        let cid = Uuid::new_v4();
        let (mut uc, tx, _) = make_use_case_with_client(cid);
        uc.active_target = ActiveTarget::Master;

        // Act
        uc.handle_event(RawInputEvent::MouseMove {
            x: 500,
            y: 500,
            time_ms: 0,
        })
        .await
        .unwrap();

        // Assert
        assert!(tx.mouse_moves.lock().unwrap().is_empty());
    }

    // ── Edge transition ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_edge_transition_at_right_edge_switches_active_target() {
        // Arrange
        let cid = Uuid::new_v4();
        let (mut uc, _tx, cursor) = make_use_case_with_client(cid);
        uc.active_target = ActiveTarget::Master;

        // Act – move cursor to within 2px of right edge of master (x=1919, width=1920)
        uc.handle_event(RawInputEvent::MouseMove {
            x: 1919,
            y: 540,
            time_ms: 0,
        })
        .await
        .unwrap();

        // Assert – routing target should now be the client
        assert_eq!(uc.get_active_target(), &ActiveTarget::Client(cid));
        // Cursor should have been teleported
        let teleports = cursor.teleport_calls.lock().unwrap();
        assert_eq!(teleports.len(), 1);
    }

    #[tokio::test]
    async fn test_update_layout_removes_client_falls_back_to_master() {
        // Arrange
        let cid = Uuid::new_v4();
        let (mut uc, _, _) = make_use_case_with_client(cid);
        uc.active_target = ActiveTarget::Client(cid);

        // Act – update layout without the client
        let new_layout = VirtualLayout::new(1920, 1080);
        uc.update_layout(new_layout);

        // Assert
        assert_eq!(uc.get_active_target(), &ActiveTarget::Master);
    }

    // ── Mouse buttons ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_mouse_button_routed_to_active_client() {
        // Arrange
        let cid = Uuid::new_v4();
        let (mut uc, tx, _) = make_use_case_with_client(cid);
        uc.active_target = ActiveTarget::Client(cid);

        // Act
        uc.handle_event(RawInputEvent::MouseButtonDown {
            button: RawMouseButton::Left,
            x: 2000,
            y: 500,
            time_ms: 0,
        })
        .await
        .unwrap();

        // Assert
        let buttons = tx.mouse_buttons.lock().unwrap();
        assert_eq!(buttons.len(), 1);
        assert_eq!(buttons[0].0, cid);
    }

    // ── Scroll ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_scroll_event_routed_to_active_client() {
        // Arrange
        let cid = Uuid::new_v4();
        let (mut uc, tx, _) = make_use_case_with_client(cid);
        uc.active_target = ActiveTarget::Client(cid);

        // Act
        uc.handle_event(RawInputEvent::MouseWheel {
            delta: 120,
            x: 2000,
            y: 500,
            time_ms: 0,
        })
        .await
        .unwrap();

        // Assert
        let scrolls = tx.mouse_scrolls.lock().unwrap();
        assert_eq!(scrolls.len(), 1);
        assert_eq!(scrolls[0].1.delta_y, 120);
    }
}
