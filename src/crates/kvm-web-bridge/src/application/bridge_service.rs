//! Core protocol translation logic.
//!
//! This module provides pure functions that translate between the two protocol
//! representations used by the bridge:
//!
//! - **Browser side**: JSON messages ([`BrowserToMasterMsg`] / [`MasterToBrowserMsg`])
//! - **Master side**: binary KVM messages ([`KvmMessage`])
//!
//! The functions in this module have no I/O side effects and no dependencies on
//! async runtimes, sockets, or threads.  This makes them easy to unit test in
//! isolation.
//!
//! # Translation directions
//!
//! ```text
//! Browser → Master:  JSON (BrowserToMasterMsg) → binary KVM (KvmMessage)
//!                    call: translate_browser_to_kvm()
//!
//! Master → Browser:  binary KVM (KvmMessage) → JSON (MasterToBrowserMsg)
//!                    call: translate_kvm_to_browser()
//! ```

use thiserror::Error;

use kvm_core::protocol::messages::{
    ClipboardDataMessage, ClipboardFormat, DisconnectReason, HelloMessage, InputEvent,
    KvmMessage, MonitorInfo, PairingResponseMessage, PlatformId, ScreenInfoMessage,
};

use crate::domain::messages::{BrowserToMasterMsg, InputEventJson, MasterToBrowserMsg};

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors that can occur during protocol translation.
///
/// These are business-logic failures (malformed input from the browser), not
/// I/O errors.  I/O errors are handled separately by the infrastructure layer.
#[derive(Debug, Error)]
pub enum BridgeError {
    /// A UUID string from the browser could not be parsed.
    ///
    /// For example, the browser sent `"client_id": "not-a-uuid"`.
    #[error("invalid UUID: {0}")]
    InvalidUuid(String),

    /// A field value was semantically invalid.
    ///
    /// For example, a capability bitmask contained reserved bits.
    #[error("invalid field value: {0}")]
    InvalidField(String),
}

// ── Browser → Master translation ─────────────────────────────────────────────

/// Translates a JSON browser message into a binary KVM protocol message.
///
/// This is the "inbound" translation path: browser → master.  It takes the
/// JSON message the browser sent and produces the binary `KvmMessage` that
/// should be forwarded to the master over TCP.
///
/// # Errors
///
/// Returns [`BridgeError::InvalidUuid`] if the browser sent a malformed UUID
/// string (in `Hello.client_id` or `PairingResponse.pairing_session_id`).
///
/// # Example
///
/// ```rust
/// use kvm_web_bridge::application::translate_browser_to_kvm;
/// use kvm_web_bridge::domain::BrowserToMasterMsg;
///
/// let msg = BrowserToMasterMsg::Disconnect;
/// let kvm = translate_browser_to_kvm(&msg).unwrap();
/// assert!(matches!(kvm, kvm_core::protocol::messages::KvmMessage::Disconnect { .. }));
/// ```
pub fn translate_browser_to_kvm(json_msg: &BrowserToMasterMsg) -> Result<KvmMessage, BridgeError> {
    match json_msg {
        BrowserToMasterMsg::Hello {
            client_id,
            client_name,
            capabilities: caps,
        } => {
            // Parse the UUID string the browser sent.
            // `uuid::Uuid::parse_str` is strict: it only accepts the canonical
            // 8-4-4-4-12 hyphenated hex format.
            let uuid = uuid::Uuid::parse_str(client_id)
                .map_err(|_| BridgeError::InvalidUuid(client_id.clone()))?;

            Ok(KvmMessage::Hello(HelloMessage {
                client_id: uuid,
                // The bridge always identifies itself as a Web platform client.
                // This tells the master which input emulation mode to use.
                protocol_version: kvm_core::protocol::messages::PROTOCOL_VERSION,
                platform_id: PlatformId::Web,
                client_name: client_name.clone(),
                capabilities: *caps,
            }))
        }

        BrowserToMasterMsg::ScreenInfo {
            width,
            height,
            scale_factor_percent,
        } => {
            // Browsers have a single "viewport" (one logical monitor).
            // We report it as the primary monitor at origin (0, 0).
            Ok(KvmMessage::ScreenInfo(ScreenInfoMessage {
                monitors: vec![MonitorInfo {
                    monitor_id: 0,
                    x_offset: 0,
                    y_offset: 0,
                    width: *width,
                    height: *height,
                    scale_factor: *scale_factor_percent,
                    is_primary: true,
                }],
            }))
        }

        BrowserToMasterMsg::PairingResponse {
            pairing_session_id,
            pin_hash,
            accepted,
        } => {
            let session_uuid = uuid::Uuid::parse_str(pairing_session_id)
                .map_err(|_| BridgeError::InvalidUuid(pairing_session_id.clone()))?;

            Ok(KvmMessage::PairingResponse(PairingResponseMessage {
                pairing_session_id: session_uuid,
                pin_hash: pin_hash.clone(),
                accepted: *accepted,
            }))
        }

        BrowserToMasterMsg::ClipboardData { text } => {
            // Browser clipboard is always plain UTF-8 text.
            // Converting the string to bytes gives us the raw payload.
            Ok(KvmMessage::ClipboardData(ClipboardDataMessage {
                format: ClipboardFormat::Utf8Text,
                data: text.as_bytes().to_vec(),
                has_more_fragments: false,
            }))
        }

        BrowserToMasterMsg::Disconnect => Ok(KvmMessage::Disconnect {
            reason: DisconnectReason::UserInitiated,
        }),

        BrowserToMasterMsg::Pong { token } => Ok(KvmMessage::Pong(*token)),
    }
}

// ── Master → Browser translation ─────────────────────────────────────────────

/// Translates a binary KVM protocol message into a JSON browser message.
///
/// This is the "outbound" translation path: master → browser.  It takes the
/// binary KVM message received from the master and produces the JSON message
/// that should be sent to the browser over the WebSocket.
///
/// # Returns
///
/// - `Some(msg)` for all messages that the browser should receive.
/// - `None` for messages that the bridge handles internally or that are
///   irrelevant to the browser (e.g., `Announce`, `AnnounceResponse`,
///   `Pong` which is handled by the keepalive loop).
pub fn translate_kvm_to_browser(kvm_msg: &KvmMessage) -> Option<MasterToBrowserMsg> {
    match kvm_msg {
        KvmMessage::HelloAck(m) => Some(MasterToBrowserMsg::HelloAck {
            accepted: m.accepted,
            reject_reason: m.reject_reason,
            server_version: m.server_version,
        }),

        KvmMessage::PairingRequest(m) => Some(MasterToBrowserMsg::PairingRequest {
            pairing_session_id: m.pairing_session_id.to_string(),
            expires_at_secs: m.expires_at_secs,
        }),

        KvmMessage::ScreenInfoAck => Some(MasterToBrowserMsg::ScreenInfoAck),

        KvmMessage::KeyEvent(m) => {
            use kvm_core::protocol::messages::KeyEventType;
            let event_type = match m.event_type {
                KeyEventType::KeyDown => "down",
                KeyEventType::KeyUp => "up",
            };
            Some(MasterToBrowserMsg::KeyEvent {
                key_code: m.key_code as u16,
                scan_code: m.scan_code,
                event_type: event_type.to_string(),
                modifiers: m.modifiers.0,
            })
        }

        KvmMessage::MouseMove(m) => Some(MasterToBrowserMsg::MouseMove {
            x: m.x,
            y: m.y,
            delta_x: m.delta_x,
            delta_y: m.delta_y,
        }),

        KvmMessage::MouseButton(m) => {
            use kvm_core::protocol::messages::ButtonEventType;
            let event_type = match m.event_type {
                ButtonEventType::Press => "press",
                ButtonEventType::Release => "release",
            };
            Some(MasterToBrowserMsg::MouseButton {
                button: m.button as u8,
                event_type: event_type.to_string(),
                x: m.x,
                y: m.y,
            })
        }

        KvmMessage::MouseScroll(m) => Some(MasterToBrowserMsg::MouseScroll {
            delta_x: m.delta_x,
            delta_y: m.delta_y,
            x: m.x,
            y: m.y,
        }),

        KvmMessage::ClipboardData(m) => {
            // Encode the raw bytes as standard base64 (RFC 4648) so they are
            // safe inside a JSON string field.  The browser decodes with `atob()`.
            let data_base64 = base64_encode(&m.data);
            let format_str = match m.format {
                ClipboardFormat::Utf8Text => "text",
                ClipboardFormat::Html => "html",
                ClipboardFormat::Image => "image",
            };
            Some(MasterToBrowserMsg::ClipboardData {
                format: format_str.to_string(),
                data_base64,
                has_more_fragments: m.has_more_fragments,
            })
        }

        KvmMessage::Disconnect { reason } => {
            let reason_str = match reason {
                DisconnectReason::UserInitiated => "user",
                DisconnectReason::ServerShutdown => "shutdown",
                DisconnectReason::ProtocolError => "protocol_error",
                DisconnectReason::Timeout => "timeout",
            };
            Some(MasterToBrowserMsg::Disconnect {
                reason: reason_str.to_string(),
            })
        }

        KvmMessage::ConfigUpdate(m) => Some(MasterToBrowserMsg::ConfigUpdate {
            log_level: m.log_level.clone(),
            disable_hotkey: m.disable_hotkey.clone(),
            flags: m.flags,
        }),

        KvmMessage::Ping(token) => Some(MasterToBrowserMsg::Ping { token: *token }),

        KvmMessage::Error(m) => Some(MasterToBrowserMsg::Error {
            error_code: m.error_code as u8,
            description: m.description.clone(),
        }),

        KvmMessage::InputBatch(events) => {
            // Translate each batch event to its JSON representation.
            // The batch structure is preserved so the browser can process
            // high-frequency events efficiently.
            let json_events: Vec<InputEventJson> =
                events.iter().map(input_event_to_json).collect();
            Some(MasterToBrowserMsg::InputBatch { events: json_events })
        }

        // Messages that are NOT forwarded to the browser:
        //
        // - Hello / ScreenInfo / PairingResponse: browser→master only
        //   (should never come from the master)
        // - Announce / AnnounceResponse: UDP discovery, not used on WebSocket
        // - Pong: handled internally by the keepalive task, not for browser
        KvmMessage::Hello(_)
        | KvmMessage::PairingResponse(_)
        | KvmMessage::ScreenInfo(_)
        | KvmMessage::Announce(_)
        | KvmMessage::AnnounceResponse(_)
        | KvmMessage::Pong(_) => None,
    }
}

// ── Helper: input event conversion ───────────────────────────────────────────

/// Converts a binary [`InputEvent`] into its JSON representation.
///
/// This is used when expanding an `InputBatch` message for forwarding to the
/// browser.  String representations are used for event types so the JavaScript
/// code is self-documenting when reading these values.
fn input_event_to_json(event: &InputEvent) -> InputEventJson {
    match event {
        InputEvent::Key(m) => {
            use kvm_core::protocol::messages::KeyEventType;
            InputEventJson::Key {
                key_code: m.key_code as u16,
                scan_code: m.scan_code,
                key_event_type: match m.event_type {
                    KeyEventType::KeyDown => "down".to_string(),
                    KeyEventType::KeyUp => "up".to_string(),
                },
                modifiers: m.modifiers.0,
            }
        }
        InputEvent::MouseMove(m) => InputEventJson::MouseMove {
            x: m.x,
            y: m.y,
            delta_x: m.delta_x,
            delta_y: m.delta_y,
        },
        InputEvent::MouseButton(m) => {
            use kvm_core::protocol::messages::ButtonEventType;
            InputEventJson::MouseButton {
                button: m.button as u8,
                button_event_type: match m.event_type {
                    ButtonEventType::Press => "press".to_string(),
                    ButtonEventType::Release => "release".to_string(),
                },
                x: m.x,
                y: m.y,
            }
        }
        InputEvent::MouseScroll(m) => InputEventJson::MouseScroll {
            delta_x: m.delta_x,
            delta_y: m.delta_y,
            x: m.x,
            y: m.y,
        },
    }
}

// ── Helper: base64 encoding ───────────────────────────────────────────────────

/// Encodes binary data as standard base64 (RFC 4648) without padding variation.
///
/// This minimal implementation avoids adding a `base64` crate dependency just
/// for this single use case (encoding clipboard binary data into JSON strings).
///
/// # Why base64?
///
/// JSON strings must be valid Unicode text.  Binary data (e.g., image bytes)
/// may contain arbitrary byte values that are not valid UTF-8.  Base64 encodes
/// every 3 raw bytes as 4 printable ASCII characters, making binary content
/// safe to embed in JSON.
///
/// # Algorithm
///
/// Input bytes are processed in 3-byte chunks.  Each chunk of 24 bits is split
/// into four 6-bit groups, and each 6-bit value is mapped to a character from
/// the 64-character alphabet `A-Za-z0-9+/`.  The final chunk is padded with
/// `=` characters if it is shorter than 3 bytes.
///
/// # Portability note
///
/// This function is pure (no side effects, no heap allocations beyond the
/// output string) and uses only the standard Rust `std::string::String` API,
/// making it trivially portable to any platform or language.
pub fn base64_encode(data: &[u8]) -> String {
    // The standard base64 alphabet as defined in RFC 4648 §4.
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    // Pre-allocate the output: every 3 input bytes map to 4 output chars,
    // rounded up.  This avoids reallocations in the loop.
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);

    // Process the input in 3-byte chunks.
    for chunk in data.chunks(3) {
        // Pad the chunk to 3 bytes with zeros if it's shorter (happens at EOF).
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        // Extract four 6-bit groups from the 24-bit concatenation b0:b1:b2.
        let i0 = (b0 >> 2) as usize;
        let i1 = (((b0 & 0x03) << 4) | (b1 >> 4)) as usize;
        let i2 = (((b1 & 0x0F) << 2) | (b2 >> 6)) as usize;
        let i3 = (b2 & 0x3F) as usize;

        // Map each 6-bit index to the alphabet character.
        result.push(ALPHABET[i0] as char);
        result.push(ALPHABET[i1] as char);
        // Add padding '=' for incomplete trailing chunks.
        result.push(if chunk.len() > 1 { ALPHABET[i2] as char } else { '=' });
        result.push(if chunk.len() > 2 { ALPHABET[i3] as char } else { '=' });
    }

    result
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kvm_core::keymap::hid::HidKeyCode;
    use kvm_core::protocol::messages::{
        ButtonEventType, ClipboardDataMessage, ClipboardFormat, ConfigUpdateMessage,
        DisconnectReason, ErrorMessage, HelloAckMessage, InputEvent, KeyEventMessage,
        KeyEventType, ModifierFlags, MouseButton, MouseButtonMessage, MouseMoveMessage,
        MouseScrollMessage, PairingRequestMessage, ProtocolErrorCode,
    };
    use uuid::Uuid;
    use kvm_core::protocol::messages::capabilities;

    // ── translate_browser_to_kvm tests ────────────────────────────────────────

    #[test]
    fn test_browser_hello_produces_hello_message() {
        // Arrange
        let client_uuid = Uuid::new_v4();
        let msg = BrowserToMasterMsg::Hello {
            client_id: client_uuid.to_string(),
            client_name: "chrome-tab".to_string(),
            capabilities: capabilities::KEYBOARD_EMULATION | capabilities::MOUSE_EMULATION,
        };

        // Act
        let result = translate_browser_to_kvm(&msg).unwrap();

        // Assert: fields are correctly mapped
        match result {
            KvmMessage::Hello(h) => {
                assert_eq!(h.client_id, client_uuid);
                assert_eq!(h.client_name, "chrome-tab");
                // Bridge always identifies as Web platform
                assert_eq!(h.platform_id, PlatformId::Web);
                assert_eq!(
                    h.capabilities,
                    capabilities::KEYBOARD_EMULATION | capabilities::MOUSE_EMULATION
                );
            }
            other => panic!("expected Hello, got {:?}", other),
        }
    }

    #[test]
    fn test_browser_hello_with_invalid_uuid_returns_error() {
        // Arrange: a UUID string that is clearly not valid
        let msg = BrowserToMasterMsg::Hello {
            client_id: "not-a-valid-uuid".to_string(),
            client_name: "test".to_string(),
            capabilities: 0,
        };

        // Act
        let result = translate_browser_to_kvm(&msg);

        // Assert: must return an error, not silently produce garbage
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BridgeError::InvalidUuid(_)));
    }

    #[test]
    fn test_browser_screen_info_produces_single_primary_monitor() {
        // Arrange
        let msg = BrowserToMasterMsg::ScreenInfo {
            width: 1920,
            height: 1080,
            scale_factor_percent: 150,
        };

        // Act
        let result = translate_browser_to_kvm(&msg).unwrap();

        // Assert: a browser always produces exactly one primary monitor
        match result {
            KvmMessage::ScreenInfo(s) => {
                assert_eq!(s.monitors.len(), 1, "browser always has exactly one monitor");
                let m = &s.monitors[0];
                assert_eq!(m.width, 1920);
                assert_eq!(m.height, 1080);
                assert_eq!(m.scale_factor, 150);
                assert!(m.is_primary, "the single browser monitor must be primary");
                assert_eq!(m.x_offset, 0);
                assert_eq!(m.y_offset, 0);
            }
            other => panic!("expected ScreenInfo, got {:?}", other),
        }
    }

    #[test]
    fn test_browser_pairing_response_accepted_is_translated() {
        // Arrange
        let session_id = Uuid::new_v4();
        let msg = BrowserToMasterMsg::PairingResponse {
            pairing_session_id: session_id.to_string(),
            pin_hash: "sha256:deadbeef01234567".to_string(),
            accepted: true,
        };

        // Act
        let result = translate_browser_to_kvm(&msg).unwrap();

        // Assert
        match result {
            KvmMessage::PairingResponse(p) => {
                assert_eq!(p.pairing_session_id, session_id);
                assert_eq!(p.pin_hash, "sha256:deadbeef01234567");
                assert!(p.accepted);
            }
            other => panic!("expected PairingResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_browser_pairing_response_rejected_is_translated() {
        // Arrange: user dismissed the dialog
        let session_id = Uuid::new_v4();
        let msg = BrowserToMasterMsg::PairingResponse {
            pairing_session_id: session_id.to_string(),
            pin_hash: String::new(),
            accepted: false,
        };

        // Act
        let result = translate_browser_to_kvm(&msg).unwrap();

        // Assert
        match result {
            KvmMessage::PairingResponse(p) => {
                assert!(!p.accepted);
                assert!(p.pin_hash.is_empty());
            }
            other => panic!("expected PairingResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_browser_pairing_response_with_invalid_uuid_returns_error() {
        let msg = BrowserToMasterMsg::PairingResponse {
            pairing_session_id: "bad-uuid".to_string(),
            pin_hash: "abc".to_string(),
            accepted: true,
        };
        let result = translate_browser_to_kvm(&msg);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BridgeError::InvalidUuid(_)));
    }

    #[test]
    fn test_browser_clipboard_produces_utf8_text_message() {
        // Arrange
        let msg = BrowserToMasterMsg::ClipboardData {
            text: "Hello, clipboard!".to_string(),
        };

        // Act
        let result = translate_browser_to_kvm(&msg).unwrap();

        // Assert
        match result {
            KvmMessage::ClipboardData(c) => {
                assert_eq!(c.format, ClipboardFormat::Utf8Text);
                assert_eq!(c.data, b"Hello, clipboard!");
                assert!(!c.has_more_fragments);
            }
            other => panic!("expected ClipboardData, got {:?}", other),
        }
    }

    #[test]
    fn test_browser_disconnect_produces_user_initiated_reason() {
        // Arrange
        let msg = BrowserToMasterMsg::Disconnect;

        // Act
        let result = translate_browser_to_kvm(&msg).unwrap();

        // Assert: browser-initiated disconnect always uses UserInitiated
        match result {
            KvmMessage::Disconnect { reason } => {
                assert_eq!(reason, DisconnectReason::UserInitiated);
            }
            other => panic!("expected Disconnect, got {:?}", other),
        }
    }

    #[test]
    fn test_browser_pong_preserves_token_value() {
        // Arrange
        let msg = BrowserToMasterMsg::Pong { token: 0xDEAD_BEEF };

        // Act
        let result = translate_browser_to_kvm(&msg).unwrap();

        // Assert: the token must be forwarded unchanged
        match result {
            KvmMessage::Pong(t) => assert_eq!(t, 0xDEAD_BEEF),
            other => panic!("expected Pong, got {:?}", other),
        }
    }

    // ── translate_kvm_to_browser tests ────────────────────────────────────────

    #[test]
    fn test_kvm_hello_ack_accepted_translates_to_json() {
        // Arrange
        let kvm = KvmMessage::HelloAck(HelloAckMessage {
            session_token: [0xAB; 32],
            server_version: 1,
            accepted: true,
            reject_reason: 0,
        });

        // Act
        let result = translate_kvm_to_browser(&kvm).unwrap();

        // Assert
        match result {
            MasterToBrowserMsg::HelloAck {
                accepted,
                reject_reason,
                server_version,
            } => {
                assert!(accepted);
                assert_eq!(reject_reason, 0);
                assert_eq!(server_version, 1);
            }
            other => panic!("expected HelloAck, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_hello_ack_rejected_preserves_reject_reason() {
        let kvm = KvmMessage::HelloAck(HelloAckMessage {
            session_token: [0u8; 32],
            server_version: 1,
            accepted: false,
            reject_reason: 0x03, // PairingRequired
        });
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::HelloAck { accepted, reject_reason, .. } => {
                assert!(!accepted);
                assert_eq!(reject_reason, 0x03);
            }
            other => panic!("expected HelloAck, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_pairing_request_translates_uuid_to_string() {
        // Arrange: UUID must be serialized to the standard hyphenated string form
        let session_id = Uuid::new_v4();
        let kvm = KvmMessage::PairingRequest(PairingRequestMessage {
            pairing_session_id: session_id,
            expires_at_secs: 1_700_000_000,
        });

        // Act
        let result = translate_kvm_to_browser(&kvm).unwrap();

        // Assert: UUID is correctly converted to string
        match result {
            MasterToBrowserMsg::PairingRequest {
                pairing_session_id,
                expires_at_secs,
            } => {
                assert_eq!(pairing_session_id, session_id.to_string());
                assert_eq!(expires_at_secs, 1_700_000_000);
            }
            other => panic!("expected PairingRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_screen_info_ack_translates_correctly() {
        let kvm = KvmMessage::ScreenInfoAck;
        let result = translate_kvm_to_browser(&kvm).unwrap();
        assert!(matches!(result, MasterToBrowserMsg::ScreenInfoAck));
    }

    #[test]
    fn test_kvm_key_event_key_down_produces_down_string() {
        let kvm = KvmMessage::KeyEvent(KeyEventMessage {
            key_code: HidKeyCode::KeyA,
            scan_code: 0x001E,
            event_type: KeyEventType::KeyDown,
            modifiers: ModifierFlags(ModifierFlags::LEFT_CTRL),
        });
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::KeyEvent { key_code, event_type, modifiers, .. } => {
                assert_eq!(key_code, HidKeyCode::KeyA as u16);
                assert_eq!(event_type, "down");
                assert_eq!(modifiers, ModifierFlags::LEFT_CTRL);
            }
            other => panic!("expected KeyEvent, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_key_event_key_up_produces_up_string() {
        let kvm = KvmMessage::KeyEvent(KeyEventMessage {
            key_code: HidKeyCode::Enter,
            scan_code: 0x001C,
            event_type: KeyEventType::KeyUp,
            modifiers: ModifierFlags::default(),
        });
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::KeyEvent { event_type, .. } => {
                assert_eq!(event_type, "up");
            }
            other => panic!("expected KeyEvent, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_mouse_move_preserves_all_fields() {
        let kvm = KvmMessage::MouseMove(MouseMoveMessage {
            x: 1920,
            y: 1080,
            delta_x: -3,
            delta_y: 7,
        });
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::MouseMove { x, y, delta_x, delta_y } => {
                assert_eq!(x, 1920);
                assert_eq!(y, 1080);
                assert_eq!(delta_x, -3);
                assert_eq!(delta_y, 7);
            }
            other => panic!("expected MouseMove, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_mouse_button_press_produces_press_string() {
        let kvm = KvmMessage::MouseButton(MouseButtonMessage {
            button: MouseButton::Left,
            event_type: ButtonEventType::Press,
            x: 100,
            y: 200,
        });
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::MouseButton { button, event_type, x, y } => {
                assert_eq!(button, MouseButton::Left as u8);
                assert_eq!(event_type, "press");
                assert_eq!(x, 100);
                assert_eq!(y, 200);
            }
            other => panic!("expected MouseButton, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_mouse_button_release_produces_release_string() {
        let kvm = KvmMessage::MouseButton(MouseButtonMessage {
            button: MouseButton::Right,
            event_type: ButtonEventType::Release,
            x: 50,
            y: 75,
        });
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::MouseButton { event_type, .. } => {
                assert_eq!(event_type, "release");
            }
            other => panic!("expected MouseButton, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_mouse_scroll_preserves_deltas() {
        let kvm = KvmMessage::MouseScroll(MouseScrollMessage {
            delta_x: 0,
            delta_y: 120,
            x: 500,
            y: 400,
        });
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::MouseScroll { delta_x, delta_y, x, y } => {
                assert_eq!(delta_x, 0);
                assert_eq!(delta_y, 120);
                assert_eq!(x, 500);
                assert_eq!(y, 400);
            }
            other => panic!("expected MouseScroll, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_clipboard_text_encodes_to_base64() {
        // Arrange: "Hello" → base64 "SGVsbG8="
        let kvm = KvmMessage::ClipboardData(ClipboardDataMessage {
            format: ClipboardFormat::Utf8Text,
            data: b"Hello".to_vec(),
            has_more_fragments: false,
        });

        // Act
        let result = translate_kvm_to_browser(&kvm).unwrap();

        // Assert
        match result {
            MasterToBrowserMsg::ClipboardData { format, data_base64, has_more_fragments } => {
                assert_eq!(format, "text");
                assert_eq!(data_base64, "SGVsbG8=");
                assert!(!has_more_fragments);
            }
            other => panic!("expected ClipboardData, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_clipboard_html_format_string() {
        let kvm = KvmMessage::ClipboardData(ClipboardDataMessage {
            format: ClipboardFormat::Html,
            data: b"<b>test</b>".to_vec(),
            has_more_fragments: false,
        });
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::ClipboardData { format, .. } => assert_eq!(format, "html"),
            other => panic!("expected ClipboardData, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_clipboard_image_format_string() {
        let kvm = KvmMessage::ClipboardData(ClipboardDataMessage {
            format: ClipboardFormat::Image,
            data: vec![0xFF, 0xD8],
            has_more_fragments: true,
        });
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::ClipboardData { format, has_more_fragments, .. } => {
                assert_eq!(format, "image");
                assert!(has_more_fragments);
            }
            other => panic!("expected ClipboardData, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_disconnect_user_initiated_produces_user_string() {
        let kvm = KvmMessage::Disconnect { reason: DisconnectReason::UserInitiated };
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::Disconnect { reason } => assert_eq!(reason, "user"),
            other => panic!("expected Disconnect, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_disconnect_server_shutdown_produces_shutdown_string() {
        let kvm = KvmMessage::Disconnect { reason: DisconnectReason::ServerShutdown };
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::Disconnect { reason } => assert_eq!(reason, "shutdown"),
            other => panic!("expected Disconnect, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_disconnect_timeout_produces_timeout_string() {
        let kvm = KvmMessage::Disconnect { reason: DisconnectReason::Timeout };
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::Disconnect { reason } => assert_eq!(reason, "timeout"),
            other => panic!("expected Disconnect, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_config_update_preserves_all_fields() {
        let kvm = KvmMessage::ConfigUpdate(ConfigUpdateMessage {
            log_level: "debug".to_string(),
            disable_hotkey: "ScrollLock+ScrollLock".to_string(),
            flags: 1,
        });
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::ConfigUpdate { log_level, disable_hotkey, flags } => {
                assert_eq!(log_level, "debug");
                assert_eq!(disable_hotkey, "ScrollLock+ScrollLock");
                assert_eq!(flags, 1);
            }
            other => panic!("expected ConfigUpdate, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_ping_preserves_token() {
        let kvm = KvmMessage::Ping(0xDEAD_BEEF_1234_5678);
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::Ping { token } => assert_eq!(token, 0xDEAD_BEEF_1234_5678),
            other => panic!("expected Ping, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_error_preserves_error_code_and_description() {
        let kvm = KvmMessage::Error(ErrorMessage {
            error_code: ProtocolErrorCode::PairingRequired,
            description: "pairing required".to_string(),
        });
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::Error { error_code, description } => {
                assert_eq!(error_code, ProtocolErrorCode::PairingRequired as u8);
                assert_eq!(description, "pairing required");
            }
            other => panic!("expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_input_batch_translates_all_events() {
        let kvm = KvmMessage::InputBatch(vec![
            InputEvent::Key(KeyEventMessage {
                key_code: HidKeyCode::KeyA,
                scan_code: 0x1E,
                event_type: KeyEventType::KeyDown,
                modifiers: ModifierFlags::default(),
            }),
            InputEvent::MouseMove(MouseMoveMessage {
                x: 100,
                y: 200,
                delta_x: 5,
                delta_y: 0,
            }),
            InputEvent::MouseButton(MouseButtonMessage {
                button: MouseButton::Left,
                event_type: ButtonEventType::Press,
                x: 100,
                y: 200,
            }),
            InputEvent::MouseScroll(MouseScrollMessage {
                delta_x: 0,
                delta_y: -120,
                x: 100,
                y: 200,
            }),
        ]);
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::InputBatch { events } => {
                assert_eq!(events.len(), 4);
                assert!(matches!(events[0], InputEventJson::Key { .. }));
                assert!(matches!(events[1], InputEventJson::MouseMove { .. }));
                assert!(matches!(events[2], InputEventJson::MouseButton { .. }));
                assert!(matches!(events[3], InputEventJson::MouseScroll { .. }));
            }
            other => panic!("expected InputBatch, got {:?}", other),
        }
    }

    #[test]
    fn test_kvm_empty_input_batch_produces_empty_json_batch() {
        let kvm = KvmMessage::InputBatch(vec![]);
        let result = translate_kvm_to_browser(&kvm).unwrap();
        match result {
            MasterToBrowserMsg::InputBatch { events } => assert!(events.is_empty()),
            other => panic!("expected InputBatch, got {:?}", other),
        }
    }

    // Messages that are NOT forwarded to the browser

    #[test]
    fn test_kvm_hello_is_not_forwarded_to_browser() {
        use kvm_core::protocol::messages::HelloMessage;
        let kvm = KvmMessage::Hello(HelloMessage {
            client_id: Uuid::nil(),
            protocol_version: 1,
            platform_id: PlatformId::Web,
            client_name: "test".to_string(),
            capabilities: 0,
        });
        assert!(translate_kvm_to_browser(&kvm).is_none());
    }

    #[test]
    fn test_kvm_pong_is_not_forwarded_to_browser() {
        // Pong is handled by the keepalive loop internally
        let kvm = KvmMessage::Pong(42);
        assert!(
            translate_kvm_to_browser(&kvm).is_none(),
            "Pong must not be forwarded to the browser"
        );
    }

    #[test]
    fn test_kvm_announce_is_not_forwarded_to_browser() {
        use kvm_core::protocol::messages::AnnounceMessage;
        let kvm = KvmMessage::Announce(AnnounceMessage {
            client_id: Uuid::nil(),
            platform_id: PlatformId::Web,
            control_port: 24800,
            client_name: "test".to_string(),
        });
        assert!(translate_kvm_to_browser(&kvm).is_none());
    }

    // ── base64_encode tests ───────────────────────────────────────────────────

    #[test]
    fn test_base64_empty_input_produces_empty_string() {
        assert_eq!(base64_encode(&[]), "");
    }

    #[test]
    fn test_base64_hello_matches_rfc_vector() {
        // RFC 4648 test vector: "Hello" → "SGVsbG8="
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
    }

    #[test]
    fn test_base64_man_no_padding() {
        // 3 bytes → 4 chars, no padding needed
        assert_eq!(base64_encode(b"Man"), "TWFu");
    }

    #[test]
    fn test_base64_one_byte_has_two_padding_chars() {
        // 1 byte → "TQ=="
        assert_eq!(base64_encode(b"M"), "TQ==");
    }

    #[test]
    fn test_base64_two_bytes_has_one_padding_char() {
        // 2 bytes → "TWE="
        assert_eq!(base64_encode(b"Ma"), "TWE=");
    }

    #[test]
    fn test_base64_all_zeros() {
        assert_eq!(base64_encode(&[0, 0, 0]), "AAAA");
    }

    #[test]
    fn test_base64_all_0xff() {
        assert_eq!(base64_encode(&[0xFF, 0xFF, 0xFF]), "////");
    }

    #[test]
    fn test_base64_output_length_always_multiple_of_four() {
        // Base64 output length is always a multiple of 4 (padding with '=').
        for len in 0..=10usize {
            let input: Vec<u8> = (0..len).map(|i| i as u8).collect();
            let encoded = base64_encode(&input);
            if !encoded.is_empty() {
                assert_eq!(
                    encoded.len() % 4,
                    0,
                    "output length {} not multiple of 4 for input len {}",
                    encoded.len(),
                    len
                );
            }
        }
    }

    // ── Full pipeline tests ───────────────────────────────────────────────────

    #[test]
    fn test_full_pipeline_browser_hello_through_kvm_codec() {
        // Verify that a browser Hello survives the complete translate → encode → decode cycle.
        use kvm_core::protocol::codec::{decode_message, encode_message};

        // Arrange
        let client_uuid = Uuid::new_v4();
        let json_input = format!(
            r#"{{"type":"Hello","client_id":"{}","client_name":"chrome-tab","capabilities":3}}"#,
            client_uuid
        );

        // Act: parse JSON → translate → encode → decode
        let browser_msg: BrowserToMasterMsg = serde_json::from_str(&json_input).unwrap();
        let kvm_msg = translate_browser_to_kvm(&browser_msg).unwrap();
        let bytes = encode_message(&kvm_msg, 0, 0).unwrap();
        let (decoded_kvm, consumed) = decode_message(&bytes).unwrap();

        // Assert: full round trip preserves all fields
        assert_eq!(consumed, bytes.len());
        match decoded_kvm {
            KvmMessage::Hello(h) => {
                assert_eq!(h.client_id, client_uuid);
                assert_eq!(h.client_name, "chrome-tab");
                assert_eq!(h.platform_id, PlatformId::Web);
            }
            other => panic!("expected Hello, got {:?}", other),
        }
    }

    #[test]
    fn test_full_pipeline_kvm_key_event_through_json() {
        // Verify that a binary KVM KeyEvent survives translate → serialize → deserialize.

        // Arrange
        let kvm_msg = KvmMessage::KeyEvent(KeyEventMessage {
            key_code: HidKeyCode::Enter,
            scan_code: 0x001C,
            event_type: KeyEventType::KeyDown,
            modifiers: ModifierFlags(ModifierFlags::LEFT_ALT),
        });

        // Act: translate → serialize to JSON → deserialize
        let json_msg = translate_kvm_to_browser(&kvm_msg).unwrap();
        let json_str = serde_json::to_string(&json_msg).unwrap();
        let decoded: MasterToBrowserMsg = serde_json::from_str(&json_str).unwrap();

        // Assert: fields survive the full trip
        assert_eq!(json_msg, decoded);
        match decoded {
            MasterToBrowserMsg::KeyEvent { event_type, modifiers, .. } => {
                assert_eq!(event_type, "down");
                assert_eq!(modifiers, ModifierFlags::LEFT_ALT);
            }
            other => panic!("expected KeyEvent, got {:?}", other),
        }
    }
}
