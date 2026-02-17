//! Integration tests for the kvm-core protocol codec.
//!
//! # Purpose
//!
//! These tests verify that every `KvmMessage` variant can be:
//!
//! 1. **Encoded** into a byte buffer (`encode_message`) without error.
//! 2. **Decoded** from that same byte buffer (`decode_message`) without error.
//! 3. **Equal** to the original message after the round trip.
//!
//! This exercises the codec, all message type serializers/deserializers, and
//! the `SequenceCounter` together through the public API exposed by `kvm-core`.
//!
//! # Why integration tests and not unit tests?
//!
//! Unit tests in `src/protocol/codec.rs` focus on individual encode/decode
//! functions.  These integration tests live in `tests/` (outside `src/`) and
//! can only access the *public* API, which is the same API used by `kvm-master`
//! and `kvm-client`.  If a type or function is mistakenly made private, these
//! tests will fail to compile, giving an early warning.
//!
//! # How round-trip testing works
//!
//! The helper function `roundtrip(msg)`:
//! 1. Creates a fresh `SequenceCounter` starting at 0.
//! 2. Calls `encode_message(&msg, seq, timestamp_us)` → `Vec<u8>`.
//! 3. Calls `decode_message(&bytes)` → `(KvmMessage, consumed_bytes)`.
//! 4. Asserts that `consumed == bytes.len()` (no bytes left over).
//! 5. Returns the decoded message.
//!
//! Each test then checks `original == decoded`.  Because `KvmMessage` derives
//! `PartialEq`, this comparison checks every field recursively.
//!
//! # Binary wire format reminder
//!
//! ```text
//! Offset  Size  Field
//! ──────  ────  ─────────────────────────────────
//!  0       1    Protocol version (currently 0x01)
//!  1       1    Message type discriminant
//!  2       2    Reserved (must be 0x0000)
//!  4       4    Payload length (big-endian u32)
//!  8       8    Sequence number (big-endian u64)
//! 16       8    Timestamp in microseconds (big-endian u64)
//! 24       N    Payload bytes (MessagePack encoded)
//! ```
//!
//! The `test_sequence_counter_increments_across_encodes` test verifies the
//! header layout by reading the sequence field directly from the raw bytes.

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

/// Encodes `msg` into bytes, decodes those bytes, and returns the decoded
/// message.
///
/// # Panics
///
/// Panics (via `expect`/`assert_eq`) if encoding fails, decoding fails, or
/// the number of consumed bytes does not match the total buffer length.
/// Any such panic means the codec has a bug.
///
/// # Example
///
/// ```rust,ignore
/// let original = KvmMessage::Ping(42);
/// let decoded = roundtrip(original.clone());
/// assert_eq!(original, decoded);
/// ```
fn roundtrip(msg: KvmMessage) -> KvmMessage {
    // Create a fresh sequence counter so each roundtrip starts from 0.
    // In production, a single counter is shared across all messages on a
    // connection to provide ordering guarantees.
    let counter = SequenceCounter::new();

    // Encode the message.  seq = counter.next() returns 0 for the first call.
    // timestamp_us = 12345 is an arbitrary value; the decoder ignores it for
    // equality comparisons (timestamp is not stored in KvmMessage fields).
    let bytes = encode_message(&msg, counter.next(), 12345).expect("encode must succeed");

    // Decode the bytes back into a KvmMessage.
    // `consumed` is how many bytes were read from `bytes`.
    let (decoded, consumed) = decode_message(&bytes).expect("decode must succeed");

    // Every byte in the buffer must be part of the message.  If `consumed <
    // bytes.len()`, the decoder left trailing bytes which indicates a framing
    // or length mismatch bug.
    assert_eq!(consumed, bytes.len(), "all bytes must be consumed");

    decoded
}

// ── Round-trip tests ──────────────────────────────────────────────────────────
//
// Each test follows the same pattern:
//   1. Build an `original` KvmMessage with representative field values.
//   2. Call `roundtrip(original.clone())` to get the decoded message.
//   3. Assert `original == decoded`.
//
// Tests use `Uuid::new_v4()` to generate random UUIDs.  This ensures the UUID
// bytes are correctly serialised and deserialised rather than accidentally
// comparing two default/zero values.

#[test]
fn test_roundtrip_hello_message() {
    // Hello is sent by the client as the first message after TCP connection.
    // `capabilities` is a bitmask; 0b0011 means "keyboard + mouse emulation".
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
    // HelloAck is the master's response to Hello.
    // `session_token` is 32 bytes; [0xAB; 32] fills every byte with 0xAB.
    // `reject_reason: 0` means "no rejection" (accepted = true).
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
    // PairingRequest is sent to the client when the user initiates pairing
    // from the master UI.  `expires_at_secs` is a Unix timestamp.
    let original = KvmMessage::PairingRequest(PairingRequestMessage {
        pairing_session_id: Uuid::new_v4(),
        expires_at_secs: 9999,
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_pairing_response_message() {
    // PairingResponse is the client's PIN submission.
    // `pin_hash` is a hex-encoded hash of the PIN and session ID.
    let original = KvmMessage::PairingResponse(PairingResponseMessage {
        pairing_session_id: Uuid::new_v4(),
        pin_hash: "sha256:aabbccdd".to_string(),
        accepted: true,
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_screen_info_message() {
    // ScreenInfo is sent by the client after Hello to tell the master about
    // its monitor layout.  Only one monitor is described here for simplicity.
    let original = KvmMessage::ScreenInfo(ScreenInfoMessage {
        monitors: vec![MonitorInfo {
            monitor_id: 0,
            x_offset: 0,
            y_offset: 0,
            width: 1920,
            height: 1080,
            // scale_factor is in percent (100 = no scaling, 150 = 150% HiDPI)
            scale_factor: 100,
            is_primary: true,
        }],
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_screen_info_ack() {
    // ScreenInfoAck is a zero-payload acknowledgement from the master to the
    // client confirming that the screen info has been received and processed.
    let original = KvmMessage::ScreenInfoAck;
    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_ping_and_pong() {
    // Ping/Pong are used for latency measurement.  The master sends a Ping
    // with a 64-bit payload; the client echoes it back as a Pong with the
    // same payload so the round-trip time can be calculated.
    // 0xDEAD_BEEF_1234_5678 is a recognisable test value.
    let ping = KvmMessage::Ping(0xDEAD_BEEF_1234_5678);
    let pong = KvmMessage::Pong(0xDEAD_BEEF_1234_5678);

    assert_eq!(ping, roundtrip(ping.clone()));
    assert_eq!(pong, roundtrip(pong.clone()));
}

#[test]
fn test_roundtrip_disconnect_message() {
    // Disconnect is sent by either party before closing the TCP connection.
    // `UserInitiated` means the user deliberately stopped sharing.
    let original = KvmMessage::Disconnect {
        reason: DisconnectReason::UserInitiated,
    };

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_error_message() {
    // Error messages carry a code (for programmatic handling) and a human-
    // readable description.  `AuthenticationFailed` is returned when a wrong
    // PIN is submitted too many times.
    let original = KvmMessage::Error(ErrorMessage {
        error_code: ProtocolErrorCode::AuthenticationFailed,
        description: "bad PIN".to_string(),
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_clipboard_data_message() {
    // ClipboardData carries clipboard contents across machines.
    // Large content is split into fragments; `has_more_fragments: false`
    // indicates this is the final (or only) fragment.
    let original = KvmMessage::ClipboardData(ClipboardDataMessage {
        format: ClipboardFormat::Utf8Text,
        data: b"Hello, world!".to_vec(),
        has_more_fragments: false,
    });

    assert_eq!(original, roundtrip(original.clone()));
}

#[test]
fn test_roundtrip_key_event_message() {
    // KeyEvent is the most common input message during text entry.
    // `scan_code: 0x1E` is the PS/2 scan code for 'A'.
    // `ModifierFlags::LEFT_SHIFT` means the left Shift key is held.
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
    // MouseMove carries both absolute position (`x`, `y`) and relative
    // movement (`delta_x`, `delta_y`) to accommodate platforms that prefer
    // one or the other.  Negative delta means movement to the left/up.
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
    // MouseButton carries the button identity, press/release, and the cursor
    // position at the time of the click.
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
    // MouseScroll uses signed deltas: negative `delta_y` = scroll down.
    // -120 is the standard Windows wheel delta for one notch downward.
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
    // InputBatch groups multiple events to amortise TCP framing overhead.
    // This batch contains one key event followed by one mouse move, which
    // is realistic for a user typing while simultaneously moving the mouse.
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
    // Announce is broadcast over UDP during client discovery.
    // `control_port: 24800` is the default TCP port the client listens on.
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
    // AnnounceResponse is the master's UDP reply to a client's Announce.
    // `already_paired: false` means the client has not yet been paired and
    // should initiate the pairing flow.
    let original = KvmMessage::AnnounceResponse(AnnounceResponseMessage {
        master_control_port: 24800,
        already_paired: false,
    });

    assert_eq!(original, roundtrip(original.clone()));
}

// ── Sequence counter test ─────────────────────────────────────────────────────

#[test]
fn test_sequence_counter_increments_across_encodes() {
    // This test verifies that `SequenceCounter::next()` produces monotonically
    // increasing values AND that `encode_message` embeds them in the correct
    // header position.
    //
    // Wire format: sequence number is a big-endian u64 at bytes 8..16.
    // (See the module doc comment for the full header layout.)
    //
    // If the counter is broken (e.g., always returns 0), the master and client
    // cannot detect out-of-order or replayed packets.

    let counter = SequenceCounter::new();
    let msg = KvmMessage::Ping(0);

    // Encode twice with consecutive sequence numbers.
    let bytes1 = encode_message(&msg, counter.next(), 0).expect("encode 1");
    let bytes2 = encode_message(&msg, counter.next(), 0).expect("encode 2");

    // Decode both to make sure they are valid packets (not just raw bytes).
    let (_, _) = decode_message(&bytes1).expect("decode 1");
    let (_, _) = decode_message(&bytes2).expect("decode 2");

    // Extract the sequence numbers directly from the raw header bytes.
    // `bytes[8..16]` is the 8-byte big-endian sequence number field.
    // `.try_into().unwrap()` converts `&[u8]` to `[u8; 8]` for `from_be_bytes`.
    let seq1 = u64::from_be_bytes(bytes1[8..16].try_into().unwrap());
    let seq2 = u64::from_be_bytes(bytes2[8..16].try_into().unwrap());

    assert_eq!(seq1, 0, "first sequence must be 0");
    assert_eq!(seq2, 1, "second sequence must be 1");
}
