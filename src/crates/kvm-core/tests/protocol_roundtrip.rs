//! Integration tests for the kvm-core protocol codec.
//!
//! These tests verify complete round-trip encoding and decoding of every
//! message type through the public API, exercising the codec, message types,
//! and sequence counter together.

use kvm_core::{
    decode_message, encode_message,
    keymap::hid::HidKeyCode,
    protocol::{
        messages::{
            AnnounceMessage, AnnounceResponseMessage, ButtonEventType, ClipboardDataMessage,
            ClipboardFormat, DisconnectReason, ErrorMessage, HelloAckMessage, HelloMessage,
            InputEvent, KeyEventMessage, KeyEventType, ModifierFlags, MonitorInfo, MouseButton,
            MouseButtonMessage, MouseMoveMessage, MouseScrollMessage, PairingRequestMessage,
            PairingResponseMessage, PlatformId, ProtocolErrorCode, ScreenInfoMessage,
        },
        sequence::SequenceCounter,
    },
    KvmMessage,
};
use uuid::Uuid;

/// Encodes a message and then decodes it, asserting that the decoded message
/// matches the original.
fn roundtrip(msg: KvmMessage) -> KvmMessage {
    let counter = SequenceCounter::new();
    let bytes = encode_message(&msg, counter.next(), 12345).expect("encode must succeed");
    let (decoded, consumed) = decode_message(&bytes).expect("decode must succeed");
    assert_eq!(consumed, bytes.len(), "all bytes must be consumed");
    decoded
}

#[test]
fn test_roundtrip_hello_message() {
    let original = KvmMessage::Hello(HelloMessage {
        client_id: Uuid::new_v4(),
        protocol_version: 0x01,
        platform_id: PlatformId::Linux,
        client_name: "integration-test".to_string(),
        capabilities: 0b0011,
    });

    let decoded = roundtrip(original.clone());

    assert_eq!(original, decoded);
}

#[test]
fn test_roundtrip_hello_ack_message() {
    let original = KvmMessage::HelloAck(HelloAckMessage {
        session_token: [0xAB; 32],
        server_version: 0x01,
        accepted: true,
        reject_reason: 0x00,
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_pairing_request_message() {
    let original = KvmMessage::PairingRequest(PairingRequestMessage {
        pairing_session_id: Uuid::new_v4(),
        expires_at_secs: 9999,
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_pairing_response_message() {
    let original = KvmMessage::PairingResponse(PairingResponseMessage {
        pairing_session_id: Uuid::new_v4(),
        pin_hash: "sha256:aabbccdd".to_string(),
        accepted: true,
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_screen_info_message() {
    let original = KvmMessage::ScreenInfo(ScreenInfoMessage {
        monitors: vec![MonitorInfo {
            monitor_id: 0,
            x_offset: 0,
            y_offset: 0,
            width: 1920,
            height: 1080,
            scale_factor: 100,
            is_primary: true,
        }],
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_screen_info_ack() {
    let original = KvmMessage::ScreenInfoAck;
    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_ping_and_pong() {
    let ping = KvmMessage::Ping(0xDEAD_BEEF_1234_5678);
    let pong = KvmMessage::Pong(0xDEAD_BEEF_1234_5678);

    assert_eq!(ping, roundtrip(ping.clone()));
    assert_eq!(pong, roundtrip(pong.clone()));
}

#[test]
fn test_roundtrip_disconnect_message() {
    let original = KvmMessage::Disconnect {
        reason: DisconnectReason::UserInitiated,
    };

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_error_message() {
    let original = KvmMessage::Error(ErrorMessage {
        error_code: ProtocolErrorCode::AuthenticationFailed,
        description: "bad PIN".to_string(),
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_clipboard_data_message() {
    let original = KvmMessage::ClipboardData(ClipboardDataMessage {
        format: ClipboardFormat::Utf8Text,
        data: b"Hello, world!".to_vec(),
        has_more_fragments: false,
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_key_event_message() {
    let original = KvmMessage::KeyEvent(KeyEventMessage {
        key_code: HidKeyCode::KeyA,
        scan_code: 0x1E,
        event_type: KeyEventType::KeyDown,
        modifiers: ModifierFlags(ModifierFlags::LEFT_SHIFT),
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_mouse_move_message() {
    let original = KvmMessage::MouseMove(MouseMoveMessage {
        x: 640,
        y: 480,
        delta_x: -5,
        delta_y: 3,
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_mouse_button_message() {
    let original = KvmMessage::MouseButton(MouseButtonMessage {
        button: MouseButton::Left,
        event_type: ButtonEventType::Press,
        x: 100,
        y: 200,
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_mouse_scroll_message() {
    let original = KvmMessage::MouseScroll(MouseScrollMessage {
        delta_x: 0,
        delta_y: -120,
        x: 960,
        y: 540,
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_input_batch_message() {
    let original = KvmMessage::InputBatch(vec![
        InputEvent::Key(KeyEventMessage {
            key_code: HidKeyCode::KeyB,
            scan_code: 0x30,
            event_type: KeyEventType::KeyUp,
            modifiers: ModifierFlags::default(),
        }),
        InputEvent::MouseMove(MouseMoveMessage {
            x: 100,
            y: 100,
            delta_x: 1,
            delta_y: 1,
        }),
    ]);

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_announce_message() {
    let original = KvmMessage::Announce(AnnounceMessage {
        client_id: Uuid::new_v4(),
        platform_id: PlatformId::Windows,
        control_port: 24800,
        client_name: "desktop".to_string(),
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_announce_response_message() {
    let original = KvmMessage::AnnounceResponse(AnnounceResponseMessage {
        master_control_port: 24800,
        already_paired: false,
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_sequence_counter_increments_across_encodes() {
    let counter = SequenceCounter::new();
    let msg = KvmMessage::Ping(0);

    let bytes1 =
        encode_message(&msg, counter.next(), 0).expect("encode 1");
    let bytes2 =
        encode_message(&msg, counter.next(), 0).expect("encode 2");

    let (_, _) = decode_message(&bytes1).expect("decode 1");
    let (_, _) = decode_message(&bytes2).expect("decode 2");

    // Sequence numbers are embedded in the header at bytes 8..16 (big-endian u64).
    let seq1 = u64::from_be_bytes(bytes1[8..16].try_into().unwrap());
    let seq2 = u64::from_be_bytes(bytes2[8..16].try_into().unwrap());

    assert_eq!(seq1, 0, "first sequence must be 0");
    assert_eq!(seq2, 1, "second sequence must be 1");
}
