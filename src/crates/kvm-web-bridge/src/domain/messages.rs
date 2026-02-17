//! JSON message types for the browser-facing WebSocket protocol.
//!
//! The native KVM protocol uses compact binary encoding (24-byte header +
//! fixed-size binary payload).  Web browsers speak text/JSON naturally.
//! Rather than forcing the browser to implement the binary codec, the bridge
//! exposes a JSON "shadow" of the binary protocol.
//!
//! # Message flow
//!
//! ```text
//! Browser → Bridge:  JSON text frame  →  BrowserToMasterMsg
//! Bridge  → Browser: KvmMessage       →  MasterToBrowserMsg  →  JSON text frame
//! ```
//!
//! # JSON discriminant
//!
//! Every message is a JSON object with a `"type"` field that identifies the
//! variant.  All other fields are flattened into the same object.  For example:
//!
//! ```json
//! {"type":"MouseMove","x":100,"y":200,"delta_x":0,"delta_y":0}
//! ```
//!
//! Serde's `#[serde(tag = "type")]` attribute handles this automatically.
//!
//! # Why separate browser→master and master→browser message types?
//!
//! The two directions carry different information:
//!
//! - The browser *sends* control messages (Hello, PairingResponse, etc.)
//! - The master *sends* input events (KeyEvent, MouseMove, etc.)
//!
//! Using two distinct enums makes it a compile-time error to accidentally
//! send a master-only message to the browser, and vice versa.

use serde::{Deserialize, Serialize};

// ── Browser → Master messages ─────────────────────────────────────────────────

/// All messages that a browser can send to the bridge over WebSocket.
///
/// Each variant corresponds to a KVM protocol message the browser wants to
/// send to the master.  The bridge translates these to binary KVM messages
/// and forwards them on the master TCP control channel.
///
/// # Serde representation
///
/// ```json
/// {"type":"Hello","client_id":"uuid","client_name":"chrome","capabilities":3}
/// {"type":"ScreenInfo","width":1920,"height":1080,"scale_factor_percent":100}
/// {"type":"Disconnect"}
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// `tag = "type"` means serde will look for a `"type"` field in the JSON object
// to determine which enum variant to use when deserializing.
#[serde(tag = "type")]
pub enum BrowserToMasterMsg {
    /// Browser introduces itself and requests a KVM session.
    ///
    /// This must be the first message the browser sends after the WebSocket
    /// connection is established.  The bridge forwards it to the master as a
    /// binary `HELLO` message.
    Hello {
        /// UUID v4 identifying this browser client.
        ///
        /// The browser generates this once and stores it in `localStorage` so
        /// it persists across page reloads.  The master uses it to recognise
        /// returning clients.
        client_id: String,

        /// Human-readable label shown in the master's client list.
        client_name: String,

        /// Bitmask of supported capabilities.
        ///
        /// Bit 0 = keyboard emulation, bit 1 = mouse emulation,
        /// bit 2 = clipboard sharing.  See `kvm_core::protocol::messages::capabilities`.
        capabilities: u32,
    },

    /// Browser reports its viewport dimensions to the master.
    ///
    /// Sent after a successful `HelloAck` (accepted = true) and again
    /// whenever the browser window is resized.
    ScreenInfo {
        /// Viewport width in CSS pixels.
        width: u32,
        /// Viewport height in CSS pixels.
        height: u32,
        /// Device pixel ratio multiplied by 100.
        ///
        /// For example, a Retina display with a 2.0 DPR sends `200`.
        /// The master uses this to scale mouse movements correctly.
        scale_factor_percent: u16,
    },

    /// Browser submits the user-entered PIN to complete device pairing.
    ///
    /// Sent in response to a `PairingRequest` from the master.
    PairingResponse {
        /// The pairing session UUID received in the `PairingRequest`.
        pairing_session_id: String,
        /// SHA-256 hash of (PIN + pairing_session_id), as a lowercase hex string.
        ///
        /// The master verifies this by computing the same hash on its side.
        pin_hash: String,
        /// `true` if the user entered a PIN; `false` if they dismissed the dialog.
        accepted: bool,
    },

    /// Browser sends clipboard text to share with the master.
    ClipboardData {
        /// The clipboard text content (UTF-8).
        text: String,
    },

    /// Browser requests a graceful session disconnection.
    Disconnect,

    /// Browser replies to a KVM application-level Ping from the master.
    ///
    /// Note: this is the KVM protocol Pong, not the WebSocket protocol pong.
    /// WebSocket protocol pong is handled automatically by tokio-tungstenite.
    Pong {
        /// Echo token from the corresponding `Ping` message.
        token: u64,
    },
}

// ── Master → Browser messages ─────────────────────────────────────────────────

/// All messages that the bridge sends to the browser over WebSocket.
///
/// Each variant corresponds to a KVM protocol message received from the master.
/// The bridge translates binary KVM messages into these JSON structs and sends
/// them as WebSocket text frames to the browser.
///
/// # Serde representation
///
/// ```json
/// {"type":"HelloAck","accepted":true,"reject_reason":0,"server_version":1}
/// {"type":"KeyEvent","key_code":4,"scan_code":30,"event_type":"down","modifiers":0}
/// {"type":"MouseMove","x":960,"y":540,"delta_x":-3,"delta_y":7}
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MasterToBrowserMsg {
    /// Master accepted or rejected the browser's `Hello`.
    HelloAck {
        /// `true` if the master accepted the connection.
        accepted: bool,
        /// Non-zero reason code when `accepted` is `false`.
        reject_reason: u8,
        /// Protocol version the master is using.
        server_version: u8,
    },

    /// Master requests PIN-based pairing (first-time connection).
    PairingRequest {
        /// Unique pairing session UUID.  Must be echoed back in `PairingResponse`.
        pairing_session_id: String,
        /// Unix timestamp (seconds) after which the pairing request expires.
        expires_at_secs: u64,
    },

    /// Master acknowledged receipt of the browser's screen information.
    ScreenInfoAck,

    /// Master pushes a keyboard event for the browser to inject into the DOM.
    KeyEvent {
        /// USB HID Usage ID — a platform-independent key code.
        ///
        /// Examples: 4 = 'a', 40 = Enter, 43 = Tab.
        key_code: u16,
        /// Hardware scan code (informational; not needed for DOM injection).
        scan_code: u16,
        /// `"down"` when the key was pressed; `"up"` when it was released.
        event_type: String,
        /// Bitmask of active modifier keys (Ctrl, Shift, Alt, Meta).
        modifiers: u8,
    },

    /// Master pushes a mouse cursor position update.
    MouseMove {
        /// Absolute X position in the client's coordinate space (pixels).
        x: i32,
        /// Absolute Y position in the client's coordinate space (pixels).
        y: i32,
        /// Relative X movement delta since the last event.
        delta_x: i16,
        /// Relative Y movement delta since the last event.
        delta_y: i16,
    },

    /// Master pushes a mouse button press or release event.
    MouseButton {
        /// Button identifier: 1=left, 2=right, 3=middle, 4=button4, 5=button5.
        button: u8,
        /// `"press"` when the button was pushed down; `"release"` when let go.
        event_type: String,
        /// Absolute X position at the time of the click.
        x: i32,
        /// Absolute Y position at the time of the click.
        y: i32,
    },

    /// Master pushes a mouse wheel scroll event.
    MouseScroll {
        /// Horizontal scroll delta (positive = right, negative = left).
        ///
        /// Units: 1/120th of a notch (matches the Windows WHEEL_DELTA convention).
        delta_x: i16,
        /// Vertical scroll delta (positive = up/away, negative = down/towards).
        delta_y: i16,
        /// Cursor X position at the time of the scroll.
        x: i32,
        /// Cursor Y position at the time of the scroll.
        y: i32,
    },

    /// Master sends clipboard content to the browser.
    ClipboardData {
        /// Content format: `"text"`, `"html"`, or `"image"`.
        format: String,
        /// Raw content encoded as standard base64 (RFC 4648).
        ///
        /// Base64 encoding makes arbitrary binary content (e.g., images) safe
        /// to embed in a JSON string field.  The browser decodes it with `atob()`.
        data_base64: String,
        /// `true` if more fragments follow (for content larger than 64 KB).
        has_more_fragments: bool,
    },

    /// Master is gracefully closing the session.
    Disconnect {
        /// Human-readable reason string.
        ///
        /// One of: `"user"`, `"shutdown"`, `"protocol_error"`, `"timeout"`.
        reason: String,
    },

    /// Master pushes updated configuration to the browser client.
    ConfigUpdate {
        /// Desired log level: `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"`.
        log_level: String,
        /// Description of the hotkey to release input focus back to the master.
        ///
        /// For example `"ScrollLock+ScrollLock"` or `"Ctrl+Alt+F1"`.
        disable_hotkey: String,
        /// Packed boolean settings bitmask.
        flags: u32,
    },

    /// Master sends a KVM application-level keepalive ping.
    ///
    /// The browser must reply with a `Pong` message carrying the same `token`.
    Ping {
        /// Echo token — must be included unchanged in the `Pong` reply.
        token: u64,
    },

    /// Master sent a protocol-level error notification.
    Error {
        /// Numeric error code (matches `ProtocolErrorCode` in kvm-core).
        error_code: u8,
        /// Human-readable description (for logging; do not display to end users).
        description: String,
    },

    /// A batch of input events forwarded as a single message.
    ///
    /// `InputBatch` messages from the master are preserved as batches in the
    /// JSON protocol.  The browser can choose to process them individually or
    /// all at once.
    InputBatch {
        /// The individual input events in this batch.
        events: Vec<InputEventJson>,
    },
}

// ── Input event type for JSON batches ─────────────────────────────────────────

/// A single input event within a JSON [`MasterToBrowserMsg::InputBatch`].
///
/// Mirrors `kvm_core::protocol::messages::InputEvent` but uses JSON-friendly
/// types (strings for event types, `u8` for enum discriminants) so the browser
/// JavaScript code can parse and act on them without a binary codec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// `tag = "event_type"` means the discriminant field is called `"event_type"`.
#[serde(tag = "event_type")]
pub enum InputEventJson {
    /// A keyboard key event within a batch.
    Key {
        key_code: u16,
        scan_code: u16,
        /// `"down"` or `"up"`.
        key_event_type: String,
        modifiers: u8,
    },
    /// A mouse position event within a batch.
    MouseMove {
        x: i32,
        y: i32,
        delta_x: i16,
        delta_y: i16,
    },
    /// A mouse button event within a batch.
    MouseButton {
        button: u8,
        /// `"press"` or `"release"`.
        button_event_type: String,
        x: i32,
        y: i32,
    },
    /// A mouse scroll event within a batch.
    MouseScroll {
        delta_x: i16,
        delta_y: i16,
        x: i32,
        y: i32,
    },
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── BrowserToMasterMsg serialization ─────────────────────────────────────

    #[test]
    fn test_browser_hello_serializes_with_type_discriminant() {
        // Arrange
        let msg = BrowserToMasterMsg::Hello {
            client_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            client_name: "test-browser".to_string(),
            capabilities: 3,
        };

        // Act
        let json = serde_json::to_string(&msg).unwrap();

        // Assert: the `"type"` field must be present and equal to the variant name
        assert!(json.contains(r#""type":"Hello""#));
        assert!(json.contains("test-browser"));
        assert!(json.contains("550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn test_browser_hello_deserializes_from_json() {
        // Arrange: simulate what a browser would send
        let json = r#"{
            "type": "Hello",
            "client_id": "550e8400-e29b-41d4-a716-446655440000",
            "client_name": "my-browser",
            "capabilities": 3
        }"#;

        // Act
        let msg: BrowserToMasterMsg = serde_json::from_str(json).unwrap();

        // Assert: correct variant and field values
        match msg {
            BrowserToMasterMsg::Hello { client_name, capabilities, .. } => {
                assert_eq!(client_name, "my-browser");
                assert_eq!(capabilities, 3);
            }
            other => panic!("expected Hello, got {:?}", other),
        }
    }

    #[test]
    fn test_browser_screen_info_round_trips() {
        // Arrange
        let original = BrowserToMasterMsg::ScreenInfo {
            width: 2560,
            height: 1440,
            scale_factor_percent: 200,
        };

        // Act: serialize then deserialize
        let json = serde_json::to_string(&original).unwrap();
        let decoded: BrowserToMasterMsg = serde_json::from_str(&json).unwrap();

        // Assert: fields survive the round trip
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_browser_pairing_response_round_trips() {
        let original = BrowserToMasterMsg::PairingResponse {
            pairing_session_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
            pin_hash: "sha256:deadbeef".to_string(),
            accepted: true,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: BrowserToMasterMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_browser_clipboard_round_trips() {
        let original = BrowserToMasterMsg::ClipboardData {
            text: "Hello, clipboard!".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: BrowserToMasterMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_browser_disconnect_round_trips() {
        let original = BrowserToMasterMsg::Disconnect;
        let json = serde_json::to_string(&original).unwrap();
        let decoded: BrowserToMasterMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_browser_pong_round_trips() {
        let original = BrowserToMasterMsg::Pong { token: 0xDEAD_BEEF_CAFE_1234 };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: BrowserToMasterMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    // ── MasterToBrowserMsg serialization ──────────────────────────────────────

    #[test]
    fn test_master_hello_ack_accepted_round_trips() {
        let original = MasterToBrowserMsg::HelloAck {
            accepted: true,
            reject_reason: 0,
            server_version: 1,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_hello_ack_rejected_round_trips() {
        let original = MasterToBrowserMsg::HelloAck {
            accepted: false,
            reject_reason: 3,
            server_version: 1,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_pairing_request_round_trips() {
        let original = MasterToBrowserMsg::PairingRequest {
            pairing_session_id: "12345678-1234-1234-1234-123456789abc".to_string(),
            expires_at_secs: 1_700_000_000,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_screen_info_ack_round_trips() {
        let original = MasterToBrowserMsg::ScreenInfoAck;
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_key_event_down_round_trips() {
        let original = MasterToBrowserMsg::KeyEvent {
            key_code: 4,
            scan_code: 0x001E,
            event_type: "down".to_string(),
            modifiers: 0x04, // LEFT_SHIFT
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_key_event_up_round_trips() {
        let original = MasterToBrowserMsg::KeyEvent {
            key_code: 40,
            scan_code: 0x001C,
            event_type: "up".to_string(),
            modifiers: 0,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_mouse_move_round_trips() {
        let original = MasterToBrowserMsg::MouseMove {
            x: 960,
            y: 540,
            delta_x: -3,
            delta_y: 7,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_mouse_button_press_round_trips() {
        let original = MasterToBrowserMsg::MouseButton {
            button: 1,
            event_type: "press".to_string(),
            x: 100,
            y: 200,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_mouse_scroll_round_trips() {
        let original = MasterToBrowserMsg::MouseScroll {
            delta_x: 0,
            delta_y: 120,
            x: 500,
            y: 400,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_clipboard_data_round_trips() {
        let original = MasterToBrowserMsg::ClipboardData {
            format: "text".to_string(),
            data_base64: "SGVsbG8=".to_string(),
            has_more_fragments: false,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_disconnect_round_trips() {
        let original = MasterToBrowserMsg::Disconnect {
            reason: "shutdown".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_config_update_round_trips() {
        let original = MasterToBrowserMsg::ConfigUpdate {
            log_level: "debug".to_string(),
            disable_hotkey: "ScrollLock+ScrollLock".to_string(),
            flags: 1,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_ping_round_trips() {
        let original = MasterToBrowserMsg::Ping { token: 0xCAFE_BABE };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_error_round_trips() {
        let original = MasterToBrowserMsg::Error {
            error_code: 3,
            description: "pairing required".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_master_input_batch_round_trips() {
        let original = MasterToBrowserMsg::InputBatch {
            events: vec![
                InputEventJson::Key {
                    key_code: 4,
                    scan_code: 0x1E,
                    key_event_type: "down".to_string(),
                    modifiers: 0,
                },
                InputEventJson::MouseMove {
                    x: 100,
                    y: 200,
                    delta_x: 5,
                    delta_y: 0,
                },
            ],
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    // ── InputEventJson serialization ──────────────────────────────────────────

    #[test]
    fn test_input_event_key_round_trips() {
        let original = InputEventJson::Key {
            key_code: 40,
            scan_code: 0x1C,
            key_event_type: "up".to_string(),
            modifiers: 0,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: InputEventJson = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_input_event_mouse_move_round_trips() {
        let original = InputEventJson::MouseMove {
            x: -10,
            y: 20,
            delta_x: -1,
            delta_y: 2,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: InputEventJson = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_input_event_mouse_button_round_trips() {
        let original = InputEventJson::MouseButton {
            button: 2,
            button_event_type: "release".to_string(),
            x: 50,
            y: 75,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: InputEventJson = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_input_event_mouse_scroll_round_trips() {
        let original = InputEventJson::MouseScroll {
            delta_x: 0,
            delta_y: -120,
            x: 500,
            y: 300,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: InputEventJson = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_unknown_message_type_returns_error() {
        // Arrange: JSON with an unknown `type` value
        let json = r#"{"type":"Unknown","foo":"bar"}"#;

        // Act
        let result: Result<BrowserToMasterMsg, _> = serde_json::from_str(json);

        // Assert: serde must return an error for unknown variants
        assert!(result.is_err(), "Unknown type must produce a deserialization error");
    }

    #[test]
    fn test_missing_type_field_returns_error() {
        // Arrange: JSON missing the required `type` field
        let json = r#"{"client_id":"abc","client_name":"x","capabilities":0}"#;

        // Act
        let result: Result<BrowserToMasterMsg, _> = serde_json::from_str(json);

        // Assert
        assert!(result.is_err(), "Missing 'type' field must produce a deserialization error");
    }
}
