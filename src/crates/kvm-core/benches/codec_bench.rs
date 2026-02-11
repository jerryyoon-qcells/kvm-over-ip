//! Criterion benchmarks for the KVM-Over-IP binary codec.
//!
//! Measures encoding and decoding latency for all message types to verify
//! compliance with the serialization+encryption budget of 1.0ms defined in
//! the project constitution §7.1.
//!
//! Run with:
//! ```bash
//! cargo bench --package kvm-core --bench codec_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use kvm_core::keymap::hid::HidKeyCode;
use kvm_core::protocol::codec::{decode_message, encode_message};
use kvm_core::protocol::messages::{
    AnnounceMessage, AnnounceResponseMessage, ClipboardDataMessage, ClipboardFormat,
    DisconnectReason, ErrorMessage, HelloAckMessage, HelloMessage, InputEvent, KeyEventMessage,
    KeyEventType, KvmMessage, ModifierFlags, MonitorInfo, MouseButton, MouseButtonMessage,
    MouseMoveMessage, MouseScrollMessage, PairingRequestMessage, PairingResponseMessage,
    PlatformId, ProtocolErrorCode, ScreenInfoMessage,
};
use uuid::Uuid;

// ── Message fixtures ──────────────────────────────────────────────────────────

fn make_ping() -> KvmMessage {
    KvmMessage::Ping(42)
}

fn make_pong() -> KvmMessage {
    KvmMessage::Pong(42)
}

fn make_key_event() -> KvmMessage {
    KvmMessage::KeyEvent(KeyEventMessage {
        key_code: HidKeyCode::KeyA,
        scan_code: 0x1E,
        event_type: KeyEventType::KeyDown,
        modifiers: ModifierFlags(ModifierFlags::LEFT_SHIFT),
    })
}

fn make_mouse_move() -> KvmMessage {
    KvmMessage::MouseMove(MouseMoveMessage {
        x: 960,
        y: 540,
        delta_x: 10,
        delta_y: -5,
    })
}

fn make_mouse_button() -> KvmMessage {
    KvmMessage::MouseButton(MouseButtonMessage {
        button: MouseButton::Left,
        event_type: kvm_core::protocol::messages::ButtonEventType::Press,
        x: 100,
        y: 200,
    })
}

fn make_mouse_scroll() -> KvmMessage {
    KvmMessage::MouseScroll(MouseScrollMessage {
        delta_x: 0,
        delta_y: -3,
        x: 960,
        y: 540,
    })
}

fn make_hello() -> KvmMessage {
    KvmMessage::Hello(HelloMessage {
        client_id: Uuid::new_v4(),
        protocol_version: 1,
        platform_id: PlatformId::Windows,
        client_name: "benchmark-client".to_string(),
        capabilities: kvm_core::protocol::messages::capabilities::KEYBOARD_EMULATION
            | kvm_core::protocol::messages::capabilities::MOUSE_EMULATION,
    })
}

fn make_hello_ack() -> KvmMessage {
    KvmMessage::HelloAck(HelloAckMessage {
        session_token: [0xAB; 32],
        server_version: 1,
        accepted: true,
        reject_reason: 0,
    })
}

fn make_screen_info() -> KvmMessage {
    KvmMessage::ScreenInfo(ScreenInfoMessage {
        monitors: vec![
            MonitorInfo {
                monitor_id: 0,
                width: 1920,
                height: 1080,
                x_offset: 0,
                y_offset: 0,
                scale_factor: 100,
                is_primary: true,
            },
            MonitorInfo {
                monitor_id: 1,
                width: 1920,
                height: 1080,
                x_offset: 1920,
                y_offset: 0,
                scale_factor: 100,
                is_primary: false,
            },
        ],
    })
}

fn make_input_batch_10() -> KvmMessage {
    let events: Vec<InputEvent> = (0..10)
        .map(|i| {
            InputEvent::Key(KeyEventMessage {
                key_code: HidKeyCode::KeyA,
                scan_code: 0x1E,
                event_type: if i % 2 == 0 {
                    KeyEventType::KeyDown
                } else {
                    KeyEventType::KeyUp
                },
                modifiers: ModifierFlags::default(),
            })
        })
        .collect();
    KvmMessage::InputBatch(events)
}

fn make_disconnect() -> KvmMessage {
    KvmMessage::Disconnect {
        reason: DisconnectReason::UserInitiated,
    }
}

fn make_pairing_request() -> KvmMessage {
    KvmMessage::PairingRequest(PairingRequestMessage {
        pairing_session_id: Uuid::new_v4(),
        expires_at_secs: 1_700_000_000,
    })
}

fn make_pairing_response() -> KvmMessage {
    KvmMessage::PairingResponse(PairingResponseMessage {
        pairing_session_id: Uuid::new_v4(),
        pin_hash: "abcdef1234567890abcdef1234567890".to_string(),
        accepted: true,
    })
}

fn make_announce() -> KvmMessage {
    KvmMessage::Announce(AnnounceMessage {
        client_id: Uuid::new_v4(),
        platform_id: PlatformId::Linux,
        control_port: 24800,
        client_name: "client-bench".to_string(),
    })
}

fn make_announce_response() -> KvmMessage {
    KvmMessage::AnnounceResponse(AnnounceResponseMessage {
        master_control_port: 24800,
        already_paired: false,
    })
}

fn make_screen_info_ack() -> KvmMessage {
    KvmMessage::ScreenInfoAck
}

fn make_error() -> KvmMessage {
    KvmMessage::Error(ErrorMessage {
        error_code: ProtocolErrorCode::AuthenticationFailed,
        description: "benchmark error message".to_string(),
    })
}

fn make_clipboard_data() -> KvmMessage {
    KvmMessage::ClipboardData(ClipboardDataMessage {
        format: ClipboardFormat::Utf8Text,
        data: b"Hello, clipboard!".to_vec(),
        has_more_fragments: false,
    })
}

// ── Benchmark groups ──────────────────────────────────────────────────────────

/// Benchmarks `encode_message` for every message type.
fn bench_encode(c: &mut Criterion) {
    let messages: &[(&str, KvmMessage)] = &[
        ("Ping", make_ping()),
        ("Pong", make_pong()),
        ("KeyEvent", make_key_event()),
        ("MouseMove", make_mouse_move()),
        ("MouseButton", make_mouse_button()),
        ("MouseScroll", make_mouse_scroll()),
        ("Hello", make_hello()),
        ("HelloAck", make_hello_ack()),
        ("ScreenInfo", make_screen_info()),
        ("ScreenInfoAck", make_screen_info_ack()),
        ("InputBatch(10)", make_input_batch_10()),
        ("Disconnect", make_disconnect()),
        ("Error", make_error()),
        ("ClipboardData", make_clipboard_data()),
        ("PairingRequest", make_pairing_request()),
        ("PairingResponse", make_pairing_response()),
        ("Announce", make_announce()),
        ("AnnounceResponse", make_announce_response()),
    ];

    let mut group = c.benchmark_group("encode_message");
    for (name, msg) in messages {
        group.bench_with_input(BenchmarkId::new("msg", name), msg, |b, msg| {
            b.iter(|| {
                encode_message(black_box(msg), black_box(1), black_box(0))
                    .expect("encode must succeed")
            })
        });
    }
    group.finish();
}

/// Benchmarks `decode_message` for every message type (round-trip from pre-encoded bytes).
fn bench_decode(c: &mut Criterion) {
    let messages: &[(&str, KvmMessage)] = &[
        ("Ping", make_ping()),
        ("Pong", make_pong()),
        ("KeyEvent", make_key_event()),
        ("MouseMove", make_mouse_move()),
        ("MouseButton", make_mouse_button()),
        ("MouseScroll", make_mouse_scroll()),
        ("Hello", make_hello()),
        ("HelloAck", make_hello_ack()),
        ("ScreenInfo", make_screen_info()),
        ("ScreenInfoAck", make_screen_info_ack()),
        ("InputBatch(10)", make_input_batch_10()),
        ("Disconnect", make_disconnect()),
        ("Error", make_error()),
        ("ClipboardData", make_clipboard_data()),
        ("PairingRequest", make_pairing_request()),
        ("PairingResponse", make_pairing_response()),
        ("Announce", make_announce()),
        ("AnnounceResponse", make_announce_response()),
    ];

    let mut group = c.benchmark_group("decode_message");
    for (name, msg) in messages {
        let bytes = encode_message(msg, 1, 0).expect("encode must succeed for benchmark setup");
        group.bench_with_input(BenchmarkId::new("msg", name), &bytes, |b, bytes| {
            b.iter(|| decode_message(black_box(bytes)).expect("decode must succeed"))
        });
    }
    group.finish();
}

/// Benchmarks a full encode+decode round-trip for the highest-frequency message types.
fn bench_roundtrip_hot_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_decode_roundtrip");

    // KeyEvent: highest frequency during text input
    let key_msg = make_key_event();
    group.bench_function("KeyEvent", |b| {
        b.iter(|| {
            let bytes =
                encode_message(black_box(&key_msg), black_box(1), black_box(0)).unwrap();
            decode_message(black_box(&bytes)).unwrap()
        })
    });

    // MouseMove: highest frequency during mouse movement
    let mouse_msg = make_mouse_move();
    group.bench_function("MouseMove", |b| {
        b.iter(|| {
            let bytes =
                encode_message(black_box(&mouse_msg), black_box(1), black_box(0)).unwrap();
            decode_message(black_box(&bytes)).unwrap()
        })
    });

    // InputBatch(10): expected common batching scenario
    let batch_msg = make_input_batch_10();
    group.bench_function("InputBatch_10", |b| {
        b.iter(|| {
            let bytes =
                encode_message(black_box(&batch_msg), black_box(1), black_box(0)).unwrap();
            decode_message(black_box(&bytes)).unwrap()
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_encode,
    bench_decode,
    bench_roundtrip_hot_path
);
criterion_main!(benches);
