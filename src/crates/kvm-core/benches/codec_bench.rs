//! Criterion benchmarks for the KVM-Over-IP binary codec.
//!
//! # Purpose
//!
//! This file measures how fast the protocol codec can encode (serialize) and
//! decode (deserialize) every message type.  The project constitution §7.1
//! sets a budget of **1.0 ms** for the complete serialization+encryption path.
//! These benchmarks verify that `encode_message` and `decode_message` stay
//! comfortably within that budget.
//!
//! # What is Criterion?
//!
//! [Criterion](https://crates.io/crates/criterion) is a statistics-driven
//! benchmarking library for Rust.  Unlike a regular test, a benchmark runs the
//! same code thousands of times and computes the mean, standard deviation, and
//! percentiles.  It also saves results between runs so it can automatically
//! detect performance regressions.
//!
//! The two most important Criterion helpers used here:
//!
//! - `black_box(value)` — prevents the compiler from optimising the value away.
//!   Without it the compiler might notice that the result is never used and
//!   skip the computation entirely, making the benchmark measure zero work.
//!
//! - `b.iter(|| { … })` — the closure is the code being timed.  Criterion
//!   runs it in a loop and measures the elapsed wall-clock time.
//!
//! - `BenchmarkId::new("group", param)` — gives each individual benchmark a
//!   human-readable name like `encode_message/msg/KeyEvent`.
//!
//! # How to run
//!
//! ```bash
//! cargo bench --package kvm-core --bench codec_bench
//! ```
//!
//! Results are saved to `target/criterion/` as HTML reports you can open in a
//! browser.

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
//
// Each `make_*` function creates a representative instance of one message type.
// We define them as functions (rather than constants) so that UUID fields get
// fresh values each time — this is important for message types that embed
// random session identifiers, preventing the benchmark from operating on
// cached values that would skew timing results.
//
// These fixtures are designed to be realistic:
// - `make_hello()` uses a plausible client name and capability flags.
// - `make_screen_info()` uses a dual-monitor configuration (common setup).
// - `make_input_batch_10()` builds a batch of 10 alternating KeyDown/KeyUp
//   events, which is a typical real-world scenario during rapid typing.

/// Creates a Ping message carrying an arbitrary payload value.
///
/// Ping messages are the smallest possible KVM messages — just the 24-byte
/// header plus an 8-byte payload.  They set the lower bound on codec latency.
fn make_ping() -> KvmMessage {
    KvmMessage::Ping(42)
}

/// Creates a Pong message (the reply to a Ping).
///
/// Structurally identical to Ping; included separately so both message type
/// discriminants are exercised.
fn make_pong() -> KvmMessage {
    KvmMessage::Pong(42)
}

/// Creates a key-down event for the 'A' key with Left Shift held.
///
/// - `HidKeyCode::KeyA` — USB HID Usage ID for the 'A' key (0x04).
/// - `scan_code: 0x1E` — the PS/2 scan code for 'A', included for
///   compatibility with legacy applications that inspect scan codes.
/// - `ModifierFlags::LEFT_SHIFT` — indicates the left Shift key is pressed.
fn make_key_event() -> KvmMessage {
    KvmMessage::KeyEvent(KeyEventMessage {
        key_code: HidKeyCode::KeyA,
        scan_code: 0x1E,
        event_type: KeyEventType::KeyDown,
        modifiers: ModifierFlags(ModifierFlags::LEFT_SHIFT),
    })
}

/// Creates a mouse-move event at the centre of a 1920×1080 screen.
///
/// - `x`, `y` — absolute position in the client's coordinate space.
/// - `delta_x`, `delta_y` — relative motion since the last event.
fn make_mouse_move() -> KvmMessage {
    KvmMessage::MouseMove(MouseMoveMessage {
        x: 960,
        y: 540,
        delta_x: 10,
        delta_y: -5,
    })
}

/// Creates a left-mouse-button press at position (100, 200).
fn make_mouse_button() -> KvmMessage {
    KvmMessage::MouseButton(MouseButtonMessage {
        button: MouseButton::Left,
        event_type: kvm_core::protocol::messages::ButtonEventType::Press,
        x: 100,
        y: 200,
    })
}

/// Creates a vertical scroll event representing three downward notches.
///
/// Negative `delta_y` = scroll down (same convention as most platforms).
fn make_mouse_scroll() -> KvmMessage {
    KvmMessage::MouseScroll(MouseScrollMessage {
        delta_x: 0,
        delta_y: -3,
        x: 960,
        y: 540,
    })
}

/// Creates a Hello handshake message sent by a connecting client.
///
/// The `capabilities` field is a bitmask advertising which input types the
/// client can receive.  Here both `KEYBOARD_EMULATION` and `MOUSE_EMULATION`
/// bits are set.
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

/// Creates a HelloAck acknowledgement sent by the master in response to Hello.
///
/// `session_token` is a 32-byte opaque token used to authorise subsequent
/// messages on this connection.  Here it is filled with `0xAB` for simplicity.
fn make_hello_ack() -> KvmMessage {
    KvmMessage::HelloAck(HelloAckMessage {
        session_token: [0xAB; 32],
        server_version: 1,
        accepted: true,
        reject_reason: 0,
    })
}

/// Creates a ScreenInfo message describing a dual-monitor setup.
///
/// Monitor 0 is the primary display at virtual (0, 0); monitor 1 sits
/// immediately to the right at (1920, 0).  Both are 1920×1080 at 100% scale.
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

/// Creates an InputBatch containing 10 alternating KeyDown/KeyUp events.
///
/// `InputBatch` groups multiple input events into a single message to reduce
/// TCP overhead during rapid input.  This fixture simulates pressing and
/// releasing the 'A' key five times very quickly.
///
/// The even indices (0, 2, 4, …) become `KeyDown` and the odd indices become
/// `KeyUp`, producing a realistic alternating pattern.
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

/// Creates a Disconnect message with the `UserInitiated` reason.
fn make_disconnect() -> KvmMessage {
    KvmMessage::Disconnect {
        reason: DisconnectReason::UserInitiated,
    }
}

/// Creates a PairingRequest message.
///
/// `pairing_session_id` is a UUID that uniquely identifies this pairing
/// attempt so the PIN can be validated against the correct session.
/// `expires_at_secs` is a Unix timestamp after which the request is invalid.
fn make_pairing_request() -> KvmMessage {
    KvmMessage::PairingRequest(PairingRequestMessage {
        pairing_session_id: Uuid::new_v4(),
        expires_at_secs: 1_700_000_000,
    })
}

/// Creates a PairingResponse acknowledging a PIN attempt.
fn make_pairing_response() -> KvmMessage {
    KvmMessage::PairingResponse(PairingResponseMessage {
        pairing_session_id: Uuid::new_v4(),
        pin_hash: "abcdef1234567890abcdef1234567890".to_string(),
        accepted: true,
    })
}

/// Creates an Announce message broadcast by a client during UDP discovery.
fn make_announce() -> KvmMessage {
    KvmMessage::Announce(AnnounceMessage {
        client_id: Uuid::new_v4(),
        platform_id: PlatformId::Linux,
        control_port: 24800,
        client_name: "client-bench".to_string(),
    })
}

/// Creates an AnnounceResponse message sent back by the master.
fn make_announce_response() -> KvmMessage {
    KvmMessage::AnnounceResponse(AnnounceResponseMessage {
        master_control_port: 24800,
        already_paired: false,
    })
}

/// Creates a ScreenInfoAck — a zero-payload acknowledgement.
///
/// This is the smallest possible non-Ping/Pong message, useful as a
/// baseline when comparing payload-length effects on encode time.
fn make_screen_info_ack() -> KvmMessage {
    KvmMessage::ScreenInfoAck
}

/// Creates an Error message with an authentication failure code.
fn make_error() -> KvmMessage {
    KvmMessage::Error(ErrorMessage {
        error_code: ProtocolErrorCode::AuthenticationFailed,
        description: "benchmark error message".to_string(),
    })
}

/// Creates a ClipboardData message containing a short ASCII string.
///
/// `has_more_fragments: false` indicates this is a single-fragment transfer.
/// For large clipboard data the payload would be split into multiple fragments,
/// but for benchmarking purposes a single small fragment is representative.
fn make_clipboard_data() -> KvmMessage {
    KvmMessage::ClipboardData(ClipboardDataMessage {
        format: ClipboardFormat::Utf8Text,
        data: b"Hello, clipboard!".to_vec(),
        has_more_fragments: false,
    })
}

// ── Benchmark groups ──────────────────────────────────────────────────────────
//
// Criterion organises benchmarks into *groups*.  Each group produces a
// separate section in the HTML report.  The three groups here cover:
//
// 1. `bench_encode` — measures `encode_message` in isolation.
// 2. `bench_decode` — measures `decode_message` in isolation (pre-encoded
//    bytes, so only decoding work is timed).
// 3. `bench_roundtrip_hot_path` — measures the full encode+decode cycle for
//    the three most frequently used message types at runtime.

/// Benchmarks `encode_message` for every message type.
///
/// The loop iterates over a slice of `(name, KvmMessage)` pairs.  For each
/// pair, `group.bench_with_input` registers one sub-benchmark whose ID is
/// `encode_message/msg/<name>` in the report.
///
/// The inner closure calls `encode_message(msg, seq=1, timestamp_us=0)` and
/// asserts success.  The `black_box` wrappers prevent the compiler from
/// constant-folding the inputs.
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
                // Encode msg with sequence number 1 and timestamp 0.
                // `black_box` ensures the compiler cannot elide this call.
                encode_message(black_box(msg), black_box(1), black_box(0))
                    .expect("encode must succeed")
            })
        });
    }
    group.finish();
}

/// Benchmarks `decode_message` for every message type.
///
/// Pre-encoding happens **outside** the timed loop so that only decoding work
/// is measured.  The bytes are passed into `bench_with_input` as the input
/// parameter so Criterion can hash them for its input sampling.
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
        // Encode once before the loop; the timed closure only runs decode.
        let bytes = encode_message(msg, 1, 0).expect("encode must succeed for benchmark setup");
        group.bench_with_input(BenchmarkId::new("msg", name), &bytes, |b, bytes| {
            b.iter(|| decode_message(black_box(bytes)).expect("decode must succeed"))
        });
    }
    group.finish();
}

/// Benchmarks a full encode+decode round-trip for the highest-frequency message types.
///
/// On the KVM hot path, every input event from the master must be encoded and
/// then decoded on the client.  These three message types are the most common:
///
/// | Message type   | Typical rate            |
/// |----------------|-------------------------|
/// | `KeyEvent`     | Up to ~1000 events/s    |
/// | `MouseMove`    | Up to ~1000 events/s    |
/// | `InputBatch`   | Bursts during fast input|
///
/// Each sub-benchmark allocates a fresh `Vec<u8>` per iteration; if allocation
/// turns out to dominate, the encode benchmark (which also allocates) will show
/// a similar absolute time.
fn bench_roundtrip_hot_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_decode_roundtrip");

    // KeyEvent: highest frequency during text input
    let key_msg = make_key_event();
    group.bench_function("KeyEvent", |b| {
        b.iter(|| {
            let bytes = encode_message(black_box(&key_msg), black_box(1), black_box(0)).unwrap();
            decode_message(black_box(&bytes)).unwrap()
        })
    });

    // MouseMove: highest frequency during mouse movement
    let mouse_msg = make_mouse_move();
    group.bench_function("MouseMove", |b| {
        b.iter(|| {
            let bytes = encode_message(black_box(&mouse_msg), black_box(1), black_box(0)).unwrap();
            decode_message(black_box(&bytes)).unwrap()
        })
    });

    // InputBatch(10): expected common batching scenario
    let batch_msg = make_input_batch_10();
    group.bench_function("InputBatch_10", |b| {
        b.iter(|| {
            let bytes = encode_message(black_box(&batch_msg), black_box(1), black_box(0)).unwrap();
            decode_message(black_box(&bytes)).unwrap()
        })
    });

    group.finish();
}

// ── Criterion entry point ─────────────────────────────────────────────────────
//
// `criterion_group!` registers the three benchmark functions under the name
// `benches`.  `criterion_main!` expands to a `main` function that Criterion
// calls when the benchmark binary is executed.

criterion_group!(
    benches,
    bench_encode,
    bench_decode,
    bench_roundtrip_hot_path
);
criterion_main!(benches);
