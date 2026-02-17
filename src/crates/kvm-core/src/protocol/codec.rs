//! Binary codec for encoding and decoding KVM-Over-IP protocol messages.
//!
//! Wire format:
//! ```text
//! [version:1][msg_type:1][reserved:2][payload_len:4][seq:8][timestamp_us:8][payload:N]
//! ```
//! Total header size: 24 bytes. All multi-byte integers are big-endian.
//!
//! # How this codec works (for beginners)
//!
//! **Encoding** converts a typed Rust value (e.g., `KvmMessage::Ping(42)`) into
//! a flat sequence of bytes that can be sent over a TCP socket.  The process is:
//!
//! 1. Call `encode_payload` to serialise only the message-specific fields.
//! 2. Build the 24-byte header (version, type, reserved, payload_len, seq, ts).
//! 3. Concatenate header + payload into a single `Vec<u8>`.
//!
//! **Decoding** is the reverse:
//!
//! 1. Check there are at least 24 bytes.
//! 2. Read and validate the version byte.
//! 3. Read the message type byte and look it up in `MessageType`.
//! 4. Read the payload length and verify enough bytes are available.
//! 5. Call `decode_payload` to reconstruct the typed struct from the bytes.
//! 6. Return the typed message plus the number of bytes consumed.
//!
//! The "number of bytes consumed" return value lets callers implement a
//! streaming read loop: keep calling `decode_message` and advance the buffer
//! by that many bytes until there is not enough data left to form a header.
//!
//! # Endianness note
//!
//! All multi-byte integers (u16, i16, u32, i32, u64, i64) are stored in
//! **big-endian** byte order (most significant byte first).  This is the
//! network standard (RFC 791) and makes the binary format readable with a
//! hex dump without any byte-swapping surprises.  In Rust the conversion is
//! done with `.to_be_bytes()` for encoding and `T::from_be_bytes([...])` for
//! decoding.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::keymap::hid::HidKeyCode;
use crate::protocol::messages::{
    AnnounceMessage, AnnounceResponseMessage, ButtonEventType, ClipboardDataMessage,
    ClipboardFormat, ConfigUpdateMessage, DisconnectReason, ErrorMessage, HelloAckMessage,
    HelloMessage, InputEvent, KeyEventMessage, KeyEventType, KvmMessage, MessageType,
    ModifierFlags, MonitorInfo, MouseButton, MouseButtonMessage, MouseMoveMessage,
    MouseScrollMessage, PairingRequestMessage, PairingResponseMessage, PlatformId,
    ProtocolErrorCode, ScreenInfoMessage, HEADER_SIZE, PROTOCOL_VERSION,
};
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during message encoding or decoding.
///
/// These variants are returned as `Err(ProtocolError::...)` from both
/// `encode_message` and `decode_message`.  Callers should match on the
/// specific variant to decide whether to log the error, close the connection,
/// or request a retransmit.
#[derive(Debug, Error, PartialEq)]
pub enum ProtocolError {
    /// The byte slice is shorter than the minimum required length.
    ///
    /// When this occurs during streaming reads it usually means the full
    /// message has not yet arrived.  The caller should buffer the data and
    /// retry once more bytes are available.
    #[error("insufficient data: need at least {needed} bytes, got {available}")]
    InsufficientData { needed: usize, available: usize },

    /// The message type byte in the header is not a recognized value.
    ///
    /// This normally indicates a protocol version mismatch or a corrupted
    /// message.  The connection should be closed.
    #[error("unknown message type: 0x{0:02X}")]
    UnknownMessageType(u8),

    /// The protocol version in the header is not supported.
    ///
    /// Returned when byte 0 of the header is not equal to `PROTOCOL_VERSION`.
    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(u8),

    /// The payload could not be parsed (field value out of range, UTF-8 error, etc.).
    ///
    /// The string argument describes what specifically failed.  This is for
    /// logging; do not display it to end users.
    #[error("malformed payload: {0}")]
    MalformedPayload(String),

    /// The encoded payload length field does not match the actual data available.
    ///
    /// `declared` is what the header claims and `available` is what is actually
    /// present.  This indicates either a truncated message or a buggy encoder.
    #[error("payload length mismatch: header says {declared}, available is {available}")]
    PayloadLengthMismatch { declared: usize, available: usize },
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Encodes a [`KvmMessage`] into a byte vector including the 24-byte header.
///
/// The sequence number is **not** set by this function – pass a pre-incremented
/// value from a [`crate::protocol::SequenceCounter`].
///
/// The caller is responsible for maintaining the `sequence_number` counter.
/// Use [`crate::protocol::SequenceCounter::next`] to obtain a fresh value for
/// each message sent on a channel.
///
/// # Parameters
///
/// - `msg`             – The message to encode.
/// - `sequence_number` – Monotonically increasing counter value for this message.
/// - `timestamp_us`    – Microseconds since Unix epoch; use 0 in tests.
///
/// # Errors
///
/// Returns [`ProtocolError`] if serialization fails.
///
/// # Examples
///
/// ```rust
/// use kvm_core::protocol::{encode_message, decode_message};
/// use kvm_core::protocol::messages::{KvmMessage};
///
/// let msg = KvmMessage::Ping(42);
/// let bytes = encode_message(&msg, 0, 0).unwrap();
/// let (decoded, consumed) = decode_message(&bytes).unwrap();
/// assert_eq!(decoded, msg);
/// assert_eq!(consumed, bytes.len());
/// ```
pub fn encode_message(
    msg: &KvmMessage,
    sequence_number: u64,
    timestamp_us: u64,
) -> Result<Vec<u8>, ProtocolError> {
    let payload = encode_payload(msg)?;
    let payload_len = payload.len() as u32;

    let mut buf = Vec::with_capacity(HEADER_SIZE + payload.len());

    // Header: version (1) + msg_type (1) + reserved (2) + payload_len (4) +
    //         seq (8) + timestamp_us (8) = 24 bytes
    buf.push(PROTOCOL_VERSION);
    buf.push(msg.message_type() as u8);
    buf.push(0x00); // reserved
    buf.push(0x00); // reserved
    buf.extend_from_slice(&payload_len.to_be_bytes());
    buf.extend_from_slice(&sequence_number.to_be_bytes());
    buf.extend_from_slice(&timestamp_us.to_be_bytes());

    buf.extend_from_slice(&payload);
    Ok(buf)
}

/// Encodes a [`KvmMessage`] using the current system time as the timestamp.
///
/// This is a convenience wrapper around `encode_message` that automatically
/// reads the system clock.  Use this in production code; use `encode_message`
/// with a fixed timestamp only in tests that need deterministic output.
///
/// # Errors
///
/// Returns [`ProtocolError`] if serialization fails.
pub fn encode_message_now(
    msg: &KvmMessage,
    sequence_number: u64,
) -> Result<Vec<u8>, ProtocolError> {
    let timestamp_us = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64;
    encode_message(msg, sequence_number, timestamp_us)
}

/// Decodes one [`KvmMessage`] from the beginning of `bytes`.
///
/// Returns the decoded message and the total number of bytes consumed
/// (header + payload), so the caller can advance their read cursor.
///
/// # Errors
///
/// Returns [`ProtocolError`] if the bytes are malformed.
///
/// # Examples
///
/// ```rust
/// use kvm_core::protocol::{encode_message, decode_message};
/// use kvm_core::protocol::messages::KvmMessage;
///
/// let original = KvmMessage::Pong(99);
/// let bytes = encode_message(&original, 1, 0).unwrap();
/// let (decoded, n) = decode_message(&bytes).unwrap();
/// assert_eq!(decoded, original);
/// assert_eq!(n, bytes.len());
/// ```
pub fn decode_message(bytes: &[u8]) -> Result<(KvmMessage, usize), ProtocolError> {
    if bytes.len() < HEADER_SIZE {
        return Err(ProtocolError::InsufficientData {
            needed: HEADER_SIZE,
            available: bytes.len(),
        });
    }

    let version = bytes[0];
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion(version));
    }

    let msg_type_byte = bytes[1];
    let msg_type = MessageType::try_from(msg_type_byte)
        .map_err(|_| ProtocolError::UnknownMessageType(msg_type_byte))?;

    // bytes[2..4] are reserved – ignored on decode

    let payload_len = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;

    let total_needed = HEADER_SIZE + payload_len;
    if bytes.len() < total_needed {
        return Err(ProtocolError::PayloadLengthMismatch {
            declared: payload_len,
            available: bytes.len() - HEADER_SIZE,
        });
    }

    let payload = &bytes[HEADER_SIZE..HEADER_SIZE + payload_len];
    let msg = decode_payload(msg_type, payload)?;
    Ok((msg, total_needed))
}

// ── Payload encoding ──────────────────────────────────────────────────────────

/// Dispatches to the message-specific encode helper and returns the serialised
/// payload bytes.  The 24-byte header is added by the caller (`encode_message`).
fn encode_payload(msg: &KvmMessage) -> Result<Vec<u8>, ProtocolError> {
    let mut buf = Vec::new();
    match msg {
        KvmMessage::Hello(m) => encode_hello(&mut buf, m),
        KvmMessage::HelloAck(m) => encode_hello_ack(&mut buf, m),
        KvmMessage::PairingRequest(m) => encode_pairing_request(&mut buf, m),
        KvmMessage::PairingResponse(m) => encode_pairing_response(&mut buf, m),
        KvmMessage::ScreenInfo(m) => encode_screen_info(&mut buf, m),
        KvmMessage::ScreenInfoAck => {} // empty payload
        KvmMessage::Ping(token) => buf.extend_from_slice(&token.to_be_bytes()),
        KvmMessage::Pong(token) => buf.extend_from_slice(&token.to_be_bytes()),
        KvmMessage::Disconnect { reason } => buf.push(*reason as u8),
        KvmMessage::Error(m) => encode_error(&mut buf, m),
        KvmMessage::ClipboardData(m) => encode_clipboard_data(&mut buf, m),
        KvmMessage::ConfigUpdate(m) => encode_config_update(&mut buf, m),
        KvmMessage::KeyEvent(m) => encode_key_event(&mut buf, m),
        KvmMessage::MouseMove(m) => encode_mouse_move(&mut buf, m),
        KvmMessage::MouseButton(m) => encode_mouse_button(&mut buf, m),
        KvmMessage::MouseScroll(m) => encode_mouse_scroll(&mut buf, m),
        KvmMessage::InputBatch(events) => encode_input_batch(&mut buf, events),
        KvmMessage::Announce(m) => encode_announce(&mut buf, m),
        KvmMessage::AnnounceResponse(m) => encode_announce_response(&mut buf, m),
    }
    Ok(buf)
}

// ── Payload decoding ──────────────────────────────────────────────────────────

/// Dispatches to the message-specific decode helper based on the type code
/// that was read from the header.  Returns the fully-typed [`KvmMessage`].
fn decode_payload(msg_type: MessageType, payload: &[u8]) -> Result<KvmMessage, ProtocolError> {
    match msg_type {
        MessageType::Hello => decode_hello(payload).map(KvmMessage::Hello),
        MessageType::HelloAck => decode_hello_ack(payload).map(KvmMessage::HelloAck),
        MessageType::PairingRequest => {
            decode_pairing_request(payload).map(KvmMessage::PairingRequest)
        }
        MessageType::PairingResponse => {
            decode_pairing_response(payload).map(KvmMessage::PairingResponse)
        }
        MessageType::ScreenInfo => decode_screen_info(payload).map(KvmMessage::ScreenInfo),
        MessageType::ScreenInfoAck => Ok(KvmMessage::ScreenInfoAck),
        MessageType::Ping => {
            let token = read_u64(payload, 0)?;
            Ok(KvmMessage::Ping(token))
        }
        MessageType::Pong => {
            let token = read_u64(payload, 0)?;
            Ok(KvmMessage::Pong(token))
        }
        MessageType::Disconnect => {
            require_len(payload, 1, "Disconnect")?;
            let reason = DisconnectReason::try_from(payload[0]).map_err(|_| {
                ProtocolError::MalformedPayload(format!(
                    "unknown disconnect reason: {}",
                    payload[0]
                ))
            })?;
            Ok(KvmMessage::Disconnect { reason })
        }
        MessageType::Error => decode_error(payload).map(KvmMessage::Error),
        MessageType::ClipboardData => decode_clipboard_data(payload).map(KvmMessage::ClipboardData),
        // FIX (Bug 2): The previous code returned Ok(KvmMessage::ScreenInfoAck) here,
        // which is the wrong message type entirely.  A receiver would silently treat
        // every ConfigUpdate as a ScreenInfoAck, ignoring the real configuration data.
        // Now we decode the actual payload into the correct KvmMessage::ConfigUpdate.
        MessageType::ConfigUpdate => decode_config_update(payload).map(KvmMessage::ConfigUpdate),
        MessageType::KeyEvent => decode_key_event(payload).map(KvmMessage::KeyEvent),
        MessageType::MouseMove => decode_mouse_move(payload).map(KvmMessage::MouseMove),
        MessageType::MouseButton => decode_mouse_button(payload).map(KvmMessage::MouseButton),
        MessageType::MouseScroll => decode_mouse_scroll(payload).map(KvmMessage::MouseScroll),
        MessageType::InputBatch => decode_input_batch(payload).map(KvmMessage::InputBatch),
        MessageType::Announce => decode_announce(payload).map(KvmMessage::Announce),
        MessageType::AnnounceResponse => {
            decode_announce_response(payload).map(KvmMessage::AnnounceResponse)
        }
    }
}

// ── Per-message encode helpers ────────────────────────────────────────────────
//
// Each function below appends the fields of one specific message type to `buf`
// in the exact byte order defined by the wire format.  The comments above each
// function show the byte layout.
//
// Convention: `buf` is always modified in-place (appended to), never reallocated
// from scratch.  This avoids extra heap allocations.

fn encode_hello(buf: &mut Vec<u8>, m: &HelloMessage) {
    buf.extend_from_slice(m.client_id.as_bytes());
    buf.push(m.protocol_version);
    buf.push(m.platform_id as u8);
    write_length_prefixed_string(buf, &m.client_name);
    buf.extend_from_slice(&m.capabilities.to_be_bytes());
}

fn encode_hello_ack(buf: &mut Vec<u8>, m: &HelloAckMessage) {
    buf.extend_from_slice(&m.session_token);
    buf.push(m.server_version);
    buf.push(if m.accepted { 0x01 } else { 0x00 });
    buf.push(m.reject_reason);
}

fn encode_pairing_request(buf: &mut Vec<u8>, m: &PairingRequestMessage) {
    buf.extend_from_slice(m.pairing_session_id.as_bytes());
    buf.extend_from_slice(&m.expires_at_secs.to_be_bytes());
}

fn encode_pairing_response(buf: &mut Vec<u8>, m: &PairingResponseMessage) {
    buf.extend_from_slice(m.pairing_session_id.as_bytes());
    write_length_prefixed_string(buf, &m.pin_hash);
    buf.push(if m.accepted { 0x01 } else { 0x00 });
}

fn encode_screen_info(buf: &mut Vec<u8>, m: &ScreenInfoMessage) {
    buf.push(m.monitors.len() as u8);
    for monitor in &m.monitors {
        buf.push(monitor.monitor_id);
        buf.extend_from_slice(&monitor.x_offset.to_be_bytes());
        buf.extend_from_slice(&monitor.y_offset.to_be_bytes());
        buf.extend_from_slice(&monitor.width.to_be_bytes());
        buf.extend_from_slice(&monitor.height.to_be_bytes());
        buf.extend_from_slice(&monitor.scale_factor.to_be_bytes());
        buf.push(if monitor.is_primary { 0x01 } else { 0x00 });
    }
}

fn encode_error(buf: &mut Vec<u8>, m: &ErrorMessage) {
    buf.push(m.error_code as u8);
    write_length_prefixed_string(buf, &m.description);
}

fn encode_clipboard_data(buf: &mut Vec<u8>, m: &ClipboardDataMessage) {
    buf.push(m.format as u8);
    buf.extend_from_slice(&(m.data.len() as u32).to_be_bytes());
    buf.extend_from_slice(&m.data);
    buf.push(if m.has_more_fragments { 0x01 } else { 0x00 });
}

/// Encodes a [`ConfigUpdateMessage`] into `buf`.
///
/// Wire layout (all multi-byte integers are big-endian):
/// ```text
/// [log_level_len    : 2 bytes][log_level     : N bytes]
/// [disable_hotkey_len : 2 bytes][disable_hotkey : M bytes]
/// [flags            : 4 bytes]
/// ```
///
/// # Why two length-prefixed strings?
///
/// Variable-length strings cannot be stored as plain bytes without a length
/// marker; the decoder would not know where one string ends and the next
/// begins.  Prefixing each string with its 2-byte length (big-endian u16)
/// solves this without needing a sentinel character.
fn encode_config_update(buf: &mut Vec<u8>, m: &ConfigUpdateMessage) {
    // Write log_level as a 2-byte-length-prefixed UTF-8 string.
    write_length_prefixed_string(buf, &m.log_level);
    // Write disable_hotkey as a 2-byte-length-prefixed UTF-8 string.
    write_length_prefixed_string(buf, &m.disable_hotkey);
    // Write the packed boolean flags as a 4-byte big-endian unsigned integer.
    buf.extend_from_slice(&m.flags.to_be_bytes());
}

fn encode_key_event(buf: &mut Vec<u8>, m: &KeyEventMessage) {
    buf.extend_from_slice(&(m.key_code as u16).to_be_bytes());
    buf.extend_from_slice(&m.scan_code.to_be_bytes());
    buf.push(m.event_type as u8);
    buf.push(m.modifiers.0);
}

fn encode_mouse_move(buf: &mut Vec<u8>, m: &MouseMoveMessage) {
    buf.extend_from_slice(&m.x.to_be_bytes());
    buf.extend_from_slice(&m.y.to_be_bytes());
    buf.extend_from_slice(&m.delta_x.to_be_bytes());
    buf.extend_from_slice(&m.delta_y.to_be_bytes());
}

fn encode_mouse_button(buf: &mut Vec<u8>, m: &MouseButtonMessage) {
    buf.push(m.button as u8);
    buf.push(m.event_type as u8);
    buf.extend_from_slice(&m.x.to_be_bytes());
    buf.extend_from_slice(&m.y.to_be_bytes());
}

fn encode_mouse_scroll(buf: &mut Vec<u8>, m: &MouseScrollMessage) {
    buf.extend_from_slice(&m.delta_x.to_be_bytes());
    buf.extend_from_slice(&m.delta_y.to_be_bytes());
    buf.extend_from_slice(&m.x.to_be_bytes());
    buf.extend_from_slice(&m.y.to_be_bytes());
}

fn encode_input_batch(buf: &mut Vec<u8>, events: &[InputEvent]) {
    buf.extend_from_slice(&(events.len() as u16).to_be_bytes());
    for event in events {
        match event {
            InputEvent::Key(m) => {
                buf.push(0x01); // discriminant
                encode_key_event(buf, m);
            }
            InputEvent::MouseMove(m) => {
                buf.push(0x02);
                encode_mouse_move(buf, m);
            }
            InputEvent::MouseButton(m) => {
                buf.push(0x03);
                encode_mouse_button(buf, m);
            }
            InputEvent::MouseScroll(m) => {
                buf.push(0x04);
                encode_mouse_scroll(buf, m);
            }
        }
    }
}

fn encode_announce(buf: &mut Vec<u8>, m: &AnnounceMessage) {
    buf.extend_from_slice(m.client_id.as_bytes());
    buf.push(m.platform_id as u8);
    buf.extend_from_slice(&m.control_port.to_be_bytes());
    write_length_prefixed_string(buf, &m.client_name);
}

fn encode_announce_response(buf: &mut Vec<u8>, m: &AnnounceResponseMessage) {
    buf.extend_from_slice(&m.master_control_port.to_be_bytes());
    buf.push(if m.already_paired { 0x01 } else { 0x00 });
}

// ── Per-message decode helpers ────────────────────────────────────────────────
//
// Each function below reads fields from the raw payload slice `p` in the same
// order they were written by the corresponding encode helper.  Comments show
// the minimum byte count and what each range contains.
//
// All functions return `ProtocolError::MalformedPayload` if a field value is
// out of range.  They never panic on invalid input.

fn decode_hello(p: &[u8]) -> Result<HelloMessage, ProtocolError> {
    // Minimum layout: 16 (uuid) + 1 (proto ver) + 1 (platform) + 2 (name_len) + name + 4 (caps)
    require_len(p, 24, "Hello")?;
    let client_id = read_uuid(p, 0)?;
    let protocol_version = p[16];
    let platform_id = PlatformId::try_from(p[17])
        .map_err(|_| ProtocolError::MalformedPayload(format!("unknown platform: {}", p[17])))?;
    let (client_name, name_end) = read_length_prefixed_string(p, 18)?;
    let caps_off = name_end;
    require_len(p, caps_off + 4, "Hello.capabilities")?;
    let capabilities = u32::from_be_bytes([
        p[caps_off],
        p[caps_off + 1],
        p[caps_off + 2],
        p[caps_off + 3],
    ]);
    Ok(HelloMessage {
        client_id,
        protocol_version,
        platform_id,
        client_name,
        capabilities,
    })
}

fn decode_hello_ack(p: &[u8]) -> Result<HelloAckMessage, ProtocolError> {
    // 32 (token) + 1 (ver) + 1 (accepted) + 1 (reject) = 35
    require_len(p, 35, "HelloAck")?;
    let mut session_token = [0u8; 32];
    session_token.copy_from_slice(&p[0..32]);
    let server_version = p[32];
    let accepted = p[33] != 0;
    let reject_reason = p[34];
    Ok(HelloAckMessage {
        session_token,
        server_version,
        accepted,
        reject_reason,
    })
}

fn decode_pairing_request(p: &[u8]) -> Result<PairingRequestMessage, ProtocolError> {
    // 16 (uuid) + 8 (expires)
    require_len(p, 24, "PairingRequest")?;
    let pairing_session_id = read_uuid(p, 0)?;
    let expires_at_secs = read_u64(p, 16)?;
    Ok(PairingRequestMessage {
        pairing_session_id,
        expires_at_secs,
    })
}

fn decode_pairing_response(p: &[u8]) -> Result<PairingResponseMessage, ProtocolError> {
    require_len(p, 19, "PairingResponse")?;
    let pairing_session_id = read_uuid(p, 0)?;
    let (pin_hash, hash_end) = read_length_prefixed_string(p, 16)?;
    require_len(p, hash_end + 1, "PairingResponse.accepted")?;
    let accepted = p[hash_end] != 0;
    Ok(PairingResponseMessage {
        pairing_session_id,
        pin_hash,
        accepted,
    })
}

fn decode_screen_info(p: &[u8]) -> Result<ScreenInfoMessage, ProtocolError> {
    require_len(p, 1, "ScreenInfo")?;
    let monitor_count = p[0] as usize;
    // Each MonitorInfo: 1+4+4+4+4+2+1 = 20 bytes
    const MONITOR_SIZE: usize = 20;
    require_len(p, 1 + monitor_count * MONITOR_SIZE, "ScreenInfo monitors")?;
    let mut monitors = Vec::with_capacity(monitor_count);
    let mut off = 1;
    for _ in 0..monitor_count {
        let monitor_id = p[off];
        let x_offset = i32::from_be_bytes([p[off + 1], p[off + 2], p[off + 3], p[off + 4]]);
        let y_offset = i32::from_be_bytes([p[off + 5], p[off + 6], p[off + 7], p[off + 8]]);
        let width = u32::from_be_bytes([p[off + 9], p[off + 10], p[off + 11], p[off + 12]]);
        let height = u32::from_be_bytes([p[off + 13], p[off + 14], p[off + 15], p[off + 16]]);
        let scale_factor = u16::from_be_bytes([p[off + 17], p[off + 18]]);
        let is_primary = p[off + 19] != 0;
        monitors.push(MonitorInfo {
            monitor_id,
            x_offset,
            y_offset,
            width,
            height,
            scale_factor,
            is_primary,
        });
        off += MONITOR_SIZE;
    }
    Ok(ScreenInfoMessage { monitors })
}

fn decode_error(p: &[u8]) -> Result<ErrorMessage, ProtocolError> {
    require_len(p, 3, "Error")?;
    let error_code = match p[0] {
        0x01 => ProtocolErrorCode::ProtocolVersionMismatch,
        0x02 => ProtocolErrorCode::AuthenticationFailed,
        0x03 => ProtocolErrorCode::PairingRequired,
        0x04 => ProtocolErrorCode::PairingFailed,
        0x05 => ProtocolErrorCode::TooManyClients,
        0x06 => ProtocolErrorCode::RateLimited,
        0x07 => ProtocolErrorCode::InternalError,
        0x08 => ProtocolErrorCode::InvalidMessage,
        _ => ProtocolErrorCode::InternalError,
    };
    let (description, _) = read_length_prefixed_string(p, 1)?;
    Ok(ErrorMessage {
        error_code,
        description,
    })
}

fn decode_clipboard_data(p: &[u8]) -> Result<ClipboardDataMessage, ProtocolError> {
    // 1 (format) + 4 (data_len) + data + 1 (has_more)
    require_len(p, 6, "ClipboardData")?;
    let format = ClipboardFormat::try_from(p[0]).map_err(|_| {
        ProtocolError::MalformedPayload(format!("unknown clipboard format: {}", p[0]))
    })?;
    let data_len = u32::from_be_bytes([p[1], p[2], p[3], p[4]]) as usize;
    require_len(p, 1 + 4 + data_len + 1, "ClipboardData.data")?;
    let data = p[5..5 + data_len].to_vec();
    let has_more_fragments = p[5 + data_len] != 0;
    Ok(ClipboardDataMessage {
        format,
        data,
        has_more_fragments,
    })
}

/// Decodes a [`ConfigUpdateMessage`] from the raw payload bytes `p`.
///
/// Expected layout:
/// ```text
/// [log_level_len    : 2][log_level     : N]
/// [disable_hotkey_len : 2][disable_hotkey : M]
/// [flags            : 4]
/// ```
/// Minimum size: 2 + 0 + 2 + 0 + 4 = 8 bytes (both strings empty, just lengths + flags).
///
/// # Why validate the minimum length?
///
/// Without a minimum-length check, a truncated packet could cause the decoder
/// to read past the end of the buffer, which would panic in debug builds or
/// silently produce garbage in release builds.  Fail fast with a clear error
/// instead.
fn decode_config_update(p: &[u8]) -> Result<ConfigUpdateMessage, ProtocolError> {
    // Minimum payload: 2-byte log_level_len + 2-byte hotkey_len + 4-byte flags = 8 bytes.
    // (Both strings may have zero length, which is valid.)
    require_len(p, 8, "ConfigUpdate")?;

    // Read the log_level string starting at offset 0.
    let (log_level, after_log_level) = read_length_prefixed_string(p, 0)?;

    // Read the disable_hotkey string immediately after log_level.
    let (disable_hotkey, after_hotkey) = read_length_prefixed_string(p, after_log_level)?;

    // Read the 4-byte flags field immediately after disable_hotkey.
    require_len(p, after_hotkey + 4, "ConfigUpdate.flags")?;
    let flags = u32::from_be_bytes([
        p[after_hotkey],
        p[after_hotkey + 1],
        p[after_hotkey + 2],
        p[after_hotkey + 3],
    ]);

    Ok(ConfigUpdateMessage {
        log_level,
        disable_hotkey,
        flags,
    })
}

fn decode_key_event(p: &[u8]) -> Result<KeyEventMessage, ProtocolError> {
    // 2 (key_code) + 2 (scan_code) + 1 (event_type) + 1 (modifiers) = 6
    require_len(p, 6, "KeyEvent")?;
    let key_code_raw = u16::from_be_bytes([p[0], p[1]]);
    let key_code = HidKeyCode::from_u16(key_code_raw);
    let scan_code = u16::from_be_bytes([p[2], p[3]]);
    let event_type = KeyEventType::try_from(p[4]).map_err(|_| {
        ProtocolError::MalformedPayload(format!("unknown key event type: {}", p[4]))
    })?;
    let modifiers = ModifierFlags(p[5]);
    Ok(KeyEventMessage {
        key_code,
        scan_code,
        event_type,
        modifiers,
    })
}

fn decode_mouse_move(p: &[u8]) -> Result<MouseMoveMessage, ProtocolError> {
    // 4+4+2+2 = 12
    require_len(p, 12, "MouseMove")?;
    let x = i32::from_be_bytes([p[0], p[1], p[2], p[3]]);
    let y = i32::from_be_bytes([p[4], p[5], p[6], p[7]]);
    let delta_x = i16::from_be_bytes([p[8], p[9]]);
    let delta_y = i16::from_be_bytes([p[10], p[11]]);
    Ok(MouseMoveMessage {
        x,
        y,
        delta_x,
        delta_y,
    })
}

fn decode_mouse_button(p: &[u8]) -> Result<MouseButtonMessage, ProtocolError> {
    // 1+1+4+4 = 10
    require_len(p, 10, "MouseButton")?;
    let button = MouseButton::try_from(p[0])
        .map_err(|_| ProtocolError::MalformedPayload(format!("unknown mouse button: {}", p[0])))?;
    let event_type = ButtonEventType::try_from(p[1]).map_err(|_| {
        ProtocolError::MalformedPayload(format!("unknown button event type: {}", p[1]))
    })?;
    let x = i32::from_be_bytes([p[2], p[3], p[4], p[5]]);
    let y = i32::from_be_bytes([p[6], p[7], p[8], p[9]]);
    Ok(MouseButtonMessage {
        button,
        event_type,
        x,
        y,
    })
}

fn decode_mouse_scroll(p: &[u8]) -> Result<MouseScrollMessage, ProtocolError> {
    // 2+2+4+4 = 12
    require_len(p, 12, "MouseScroll")?;
    let delta_x = i16::from_be_bytes([p[0], p[1]]);
    let delta_y = i16::from_be_bytes([p[2], p[3]]);
    let x = i32::from_be_bytes([p[4], p[5], p[6], p[7]]);
    let y = i32::from_be_bytes([p[8], p[9], p[10], p[11]]);
    Ok(MouseScrollMessage {
        delta_x,
        delta_y,
        x,
        y,
    })
}

fn decode_input_batch(p: &[u8]) -> Result<Vec<InputEvent>, ProtocolError> {
    require_len(p, 2, "InputBatch")?;
    let count = u16::from_be_bytes([p[0], p[1]]) as usize;
    let mut events = Vec::with_capacity(count);
    let mut off = 2;
    for _ in 0..count {
        require_len(p, off + 1, "InputBatch discriminant")?;
        let discriminant = p[off];
        off += 1;
        let event = match discriminant {
            0x01 => {
                let m = decode_key_event(&p[off..])?;
                off += 6;
                InputEvent::Key(m)
            }
            0x02 => {
                let m = decode_mouse_move(&p[off..])?;
                off += 12;
                InputEvent::MouseMove(m)
            }
            0x03 => {
                let m = decode_mouse_button(&p[off..])?;
                off += 10;
                InputEvent::MouseButton(m)
            }
            0x04 => {
                let m = decode_mouse_scroll(&p[off..])?;
                off += 12;
                InputEvent::MouseScroll(m)
            }
            _ => {
                return Err(ProtocolError::MalformedPayload(format!(
                    "unknown InputBatch event discriminant: {discriminant}"
                )));
            }
        };
        events.push(event);
    }
    Ok(events)
}

fn decode_announce(p: &[u8]) -> Result<AnnounceMessage, ProtocolError> {
    // 16 (uuid) + 1 (platform) + 2 (port) + 2 (name_len) + name
    require_len(p, 21, "Announce")?;
    let client_id = read_uuid(p, 0)?;
    let platform_id = PlatformId::try_from(p[16])
        .map_err(|_| ProtocolError::MalformedPayload(format!("unknown platform: {}", p[16])))?;
    let control_port = u16::from_be_bytes([p[17], p[18]]);
    let (client_name, _) = read_length_prefixed_string(p, 19)?;
    Ok(AnnounceMessage {
        client_id,
        platform_id,
        control_port,
        client_name,
    })
}

fn decode_announce_response(p: &[u8]) -> Result<AnnounceResponseMessage, ProtocolError> {
    require_len(p, 3, "AnnounceResponse")?;
    let master_control_port = u16::from_be_bytes([p[0], p[1]]);
    let already_paired = p[2] != 0;
    Ok(AnnounceResponseMessage {
        master_control_port,
        already_paired,
    })
}

// ── Utility helpers ───────────────────────────────────────────────────────────

/// Checks that `buf` is at least `needed` bytes long.
///
/// Returns `Err(ProtocolError::MalformedPayload)` with a message that includes
/// `context` (the field name) if the check fails.  This makes error messages
/// like "Hello.capabilities: need 4 bytes, got 2" instead of a generic index
/// out-of-bounds panic.
fn require_len(buf: &[u8], needed: usize, context: &str) -> Result<(), ProtocolError> {
    if buf.len() < needed {
        Err(ProtocolError::MalformedPayload(format!(
            "{context}: need {needed} bytes, got {}",
            buf.len()
        )))
    } else {
        Ok(())
    }
}

/// Reads a big-endian u64 from `buf` starting at `offset`.
///
/// Returns `Err(ProtocolError::InsufficientData)` if fewer than 8 bytes are
/// available at the given offset.
fn read_u64(buf: &[u8], offset: usize) -> Result<u64, ProtocolError> {
    if buf.len() < offset + 8 {
        return Err(ProtocolError::InsufficientData {
            needed: offset + 8,
            available: buf.len(),
        });
    }
    Ok(u64::from_be_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
        buf[offset + 4],
        buf[offset + 5],
        buf[offset + 6],
        buf[offset + 7],
    ]))
}

/// Reads 16 bytes from `buf` at `offset` and interprets them as a UUID (in
/// big-endian byte order, which is the standard UUID wire format).
fn read_uuid(buf: &[u8], offset: usize) -> Result<Uuid, ProtocolError> {
    if buf.len() < offset + 16 {
        return Err(ProtocolError::MalformedPayload(format!(
            "need 16 bytes for UUID at offset {offset}, got {}",
            buf.len().saturating_sub(offset)
        )));
    }
    Ok(Uuid::from_bytes(
        buf[offset..offset + 16].try_into().unwrap(),
    ))
}

/// Writes a 2-byte length prefix followed by the UTF-8 string bytes.
///
/// This is the "Pascal string" encoding: [len_hi][len_lo][bytes...].
/// The 2-byte length field limits strings to at most 65,535 bytes.
/// Any excess is silently truncated (strings longer than 64 KiB are unusual
/// in this protocol).
fn write_length_prefixed_string(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    // Clamp to u16::MAX to guard against unreasonably long strings.
    let len = bytes.len().min(u16::MAX as usize) as u16;
    // Write the length as a big-endian 2-byte integer.
    buf.extend_from_slice(&len.to_be_bytes());
    // Write exactly `len` bytes of the string content.
    buf.extend_from_slice(&bytes[..len as usize]);
}

/// Reads a 2-byte length prefix and then that many UTF-8 bytes.
///
/// Returns the decoded string and the byte offset immediately *after* the
/// string (i.e., the index at which the next field begins).  This "cursor"
/// approach avoids the need to track a mutable position variable in the
/// caller.
///
/// # Errors
///
/// Returns [`ProtocolError::MalformedPayload`] if:
/// - Fewer than 2 bytes are available for the length prefix.
/// - The declared string length exceeds the remaining buffer.
/// - The bytes are not valid UTF-8.
fn read_length_prefixed_string(
    buf: &[u8],
    offset: usize,
) -> Result<(String, usize), ProtocolError> {
    if buf.len() < offset + 2 {
        return Err(ProtocolError::MalformedPayload(format!(
            "need 2 bytes for string length at offset {offset}"
        )));
    }
    let len = u16::from_be_bytes([buf[offset], buf[offset + 1]]) as usize;
    let start = offset + 2;
    if buf.len() < start + len {
        return Err(ProtocolError::MalformedPayload(format!(
            "string of length {len} at offset {start} exceeds buffer"
        )));
    }
    let s = std::str::from_utf8(&buf[start..start + len])
        .map_err(|e| ProtocolError::MalformedPayload(format!("invalid UTF-8: {e}")))?
        .to_string();
    Ok((s, start + len))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::hid::HidKeyCode;
    use crate::protocol::messages::*;
    use uuid::Uuid;

    fn round_trip(msg: &KvmMessage) -> KvmMessage {
        let encoded = encode_message(msg, 0, 0).expect("encode failed");
        let (decoded, consumed) = decode_message(&encoded).expect("decode failed");
        assert_eq!(
            consumed,
            encoded.len(),
            "consumed bytes should equal total encoded size"
        );
        decoded
    }

    // ── Hello ────────────────────────────────────────────────────────────────

    #[test]
    fn test_hello_round_trip() {
        let msg = KvmMessage::Hello(HelloMessage {
            client_id: Uuid::new_v4(),
            protocol_version: 1,
            platform_id: PlatformId::Linux,
            client_name: "dev-linux".to_string(),
            capabilities: capabilities::KEYBOARD_EMULATION | capabilities::MOUSE_EMULATION,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_hello_with_empty_client_name() {
        let msg = KvmMessage::Hello(HelloMessage {
            client_id: Uuid::nil(),
            protocol_version: 1,
            platform_id: PlatformId::Web,
            client_name: String::new(),
            capabilities: 0,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_hello_with_max_length_client_name() {
        let long_name = "a".repeat(u16::MAX as usize);
        let msg = KvmMessage::Hello(HelloMessage {
            client_id: Uuid::new_v4(),
            protocol_version: 1,
            platform_id: PlatformId::Windows,
            client_name: long_name,
            capabilities: 0xFF,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    // ── HelloAck ─────────────────────────────────────────────────────────────

    #[test]
    fn test_hello_ack_accepted_round_trip() {
        let msg = KvmMessage::HelloAck(HelloAckMessage {
            session_token: [0xAB; 32],
            server_version: 1,
            accepted: true,
            reject_reason: 0,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_hello_ack_rejected_round_trip() {
        let msg = KvmMessage::HelloAck(HelloAckMessage {
            session_token: [0u8; 32],
            server_version: 1,
            accepted: false,
            reject_reason: 0x03,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    // ── PairingRequest / PairingResponse ──────────────────────────────────────

    #[test]
    fn test_pairing_request_round_trip() {
        let msg = KvmMessage::PairingRequest(PairingRequestMessage {
            pairing_session_id: Uuid::new_v4(),
            expires_at_secs: 1_700_000_000,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_pairing_response_accepted_round_trip() {
        let msg = KvmMessage::PairingResponse(PairingResponseMessage {
            pairing_session_id: Uuid::new_v4(),
            pin_hash: "sha256:abcdef1234567890".to_string(),
            accepted: true,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    // ── ScreenInfo ────────────────────────────────────────────────────────────

    #[test]
    fn test_screen_info_single_monitor_round_trip() {
        let msg = KvmMessage::ScreenInfo(ScreenInfoMessage {
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
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_screen_info_multi_monitor_round_trip() {
        let msg = KvmMessage::ScreenInfo(ScreenInfoMessage {
            monitors: vec![
                MonitorInfo {
                    monitor_id: 0,
                    x_offset: 0,
                    y_offset: 0,
                    width: 2560,
                    height: 1440,
                    scale_factor: 150,
                    is_primary: true,
                },
                MonitorInfo {
                    monitor_id: 1,
                    x_offset: 2560,
                    y_offset: 0,
                    width: 1920,
                    height: 1080,
                    scale_factor: 100,
                    is_primary: false,
                },
            ],
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_screen_info_zero_monitors_round_trip() {
        let msg = KvmMessage::ScreenInfo(ScreenInfoMessage { monitors: vec![] });
        assert_eq!(round_trip(&msg), msg);
    }

    // ── ScreenInfoAck ─────────────────────────────────────────────────────────

    #[test]
    fn test_screen_info_ack_round_trip() {
        let msg = KvmMessage::ScreenInfoAck;
        assert_eq!(round_trip(&msg), msg);
    }

    // ── Ping / Pong ───────────────────────────────────────────────────────────

    #[test]
    fn test_ping_round_trip() {
        let msg = KvmMessage::Ping(0xDEAD_BEEF_1234_5678);
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_pong_round_trip() {
        let msg = KvmMessage::Pong(0);
        assert_eq!(round_trip(&msg), msg);
    }

    // ── Disconnect ────────────────────────────────────────────────────────────

    #[test]
    fn test_disconnect_user_initiated_round_trip() {
        let msg = KvmMessage::Disconnect {
            reason: DisconnectReason::UserInitiated,
        };
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_disconnect_timeout_round_trip() {
        let msg = KvmMessage::Disconnect {
            reason: DisconnectReason::Timeout,
        };
        assert_eq!(round_trip(&msg), msg);
    }

    // ── Error ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_error_message_round_trip() {
        let msg = KvmMessage::Error(ErrorMessage {
            error_code: ProtocolErrorCode::PairingRequired,
            description: "client must complete pairing before sending input".to_string(),
        });
        assert_eq!(round_trip(&msg), msg);
    }

    // ── ClipboardData ─────────────────────────────────────────────────────────

    #[test]
    fn test_clipboard_data_text_round_trip() {
        let msg = KvmMessage::ClipboardData(ClipboardDataMessage {
            format: ClipboardFormat::Utf8Text,
            data: b"Hello, world!".to_vec(),
            has_more_fragments: false,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_clipboard_data_empty_round_trip() {
        let msg = KvmMessage::ClipboardData(ClipboardDataMessage {
            format: ClipboardFormat::Html,
            data: vec![],
            has_more_fragments: false,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_clipboard_data_fragment_round_trip() {
        let msg = KvmMessage::ClipboardData(ClipboardDataMessage {
            format: ClipboardFormat::Image,
            data: vec![0xFF, 0xD8, 0xFF, 0xE0], // JPEG magic bytes
            has_more_fragments: true,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    // ── ConfigUpdate ──────────────────────────────────────────────────────────
    //
    // These tests verify the fix for Bug 2: ConfigUpdateMessage was previously
    // missing from KvmMessage, and the decoder returned the wrong variant.

    #[test]
    fn test_config_update_round_trip_typical() {
        // Arrange: a realistic config update sent from master to client.
        let msg = KvmMessage::ConfigUpdate(ConfigUpdateMessage {
            log_level: "info".to_string(),
            disable_hotkey: "ScrollLock+ScrollLock".to_string(),
            flags: config_flags::AUTOSTART,
        });

        // Act + Assert: encode then decode must produce the identical message.
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_config_update_round_trip_empty_strings() {
        // Arrange: both string fields are empty — this is a valid (minimal) payload.
        let msg = KvmMessage::ConfigUpdate(ConfigUpdateMessage {
            log_level: String::new(),
            disable_hotkey: String::new(),
            flags: 0,
        });

        // Act + Assert
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_config_update_round_trip_all_flags_set() {
        // Arrange: all 32 flag bits set — the decoder must preserve the full value.
        let msg = KvmMessage::ConfigUpdate(ConfigUpdateMessage {
            log_level: "trace".to_string(),
            disable_hotkey: "F12".to_string(),
            flags: u32::MAX,
        });

        // Act + Assert
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_config_update_round_trip_debug_log_level() {
        // Arrange: verify non-default log levels survive the wire.
        let msg = KvmMessage::ConfigUpdate(ConfigUpdateMessage {
            log_level: "debug".to_string(),
            disable_hotkey: "Ctrl+Alt+F1".to_string(),
            flags: 0,
        });

        // Act + Assert
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_config_update_message_type_byte_is_0x0c() {
        // Arrange: encode a ConfigUpdate and inspect the raw header byte.
        let msg = KvmMessage::ConfigUpdate(ConfigUpdateMessage {
            log_level: "info".to_string(),
            disable_hotkey: String::new(),
            flags: 0,
        });

        // Act
        let bytes = encode_message(&msg, 0, 0).unwrap();

        // Assert: byte[1] of the header is the message type — must be 0x0C.
        // This confirms the variant is wired to the correct MessageType discriminant.
        assert_eq!(
            bytes[1], 0x0C,
            "ConfigUpdate must encode with message type 0x0C"
        );
    }

    #[test]
    fn test_config_update_decode_truncated_payload_returns_error() {
        // Arrange: build a syntactically valid header that claims a 2-byte payload,
        // but a ConfigUpdate needs at least 8 bytes (two empty string prefixes + flags).
        let mut bytes = vec![0u8; 24 + 2]; // header + 2-byte payload
        bytes[0] = PROTOCOL_VERSION;
        bytes[1] = 0x0C; // ConfigUpdate
                         // payload_length field is bytes[4..8]
        let payload_len = 2u32;
        bytes[4..8].copy_from_slice(&payload_len.to_be_bytes());
        // Remaining bytes are zero (the "payload"), which is only 2 bytes.

        // Act
        let result = decode_message(&bytes);

        // Assert: the decoder must return an error, not silently produce garbage.
        assert!(
            result.is_err(),
            "decoding a truncated ConfigUpdate payload must return an error"
        );
    }

    #[test]
    fn test_config_update_decode_returns_config_update_variant_not_screen_info_ack() {
        // Arrange: this test specifically guards against the regression where the
        // decoder returned KvmMessage::ScreenInfoAck for a ConfigUpdate message.
        let original = KvmMessage::ConfigUpdate(ConfigUpdateMessage {
            log_level: "warn".to_string(),
            disable_hotkey: "ScrollLock".to_string(),
            flags: 0,
        });
        let bytes = encode_message(&original, 0, 0).unwrap();

        // Act
        let (decoded, _) = decode_message(&bytes).unwrap();

        // Assert: must be ConfigUpdate, never ScreenInfoAck.
        assert!(
            matches!(decoded, KvmMessage::ConfigUpdate(_)),
            "ConfigUpdate bytes must decode to KvmMessage::ConfigUpdate, not {:?}",
            decoded
        );
    }

    // ── KeyEvent ──────────────────────────────────────────────────────────────

    #[test]
    fn test_key_event_key_down_round_trip() {
        let msg = KvmMessage::KeyEvent(KeyEventMessage {
            key_code: HidKeyCode::KeyA,
            scan_code: 0x001E,
            event_type: KeyEventType::KeyDown,
            modifiers: ModifierFlags(ModifierFlags::LEFT_SHIFT),
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_key_event_key_up_round_trip() {
        let msg = KvmMessage::KeyEvent(KeyEventMessage {
            key_code: HidKeyCode::Enter,
            scan_code: 0x001C,
            event_type: KeyEventType::KeyUp,
            modifiers: ModifierFlags::default(),
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_key_event_all_modifiers_set() {
        let msg = KvmMessage::KeyEvent(KeyEventMessage {
            key_code: HidKeyCode::KeyA,
            scan_code: 0x001E,
            event_type: KeyEventType::KeyDown,
            modifiers: ModifierFlags(0xFF),
        });
        assert_eq!(round_trip(&msg), msg);
    }

    // ── MouseMove ─────────────────────────────────────────────────────────────

    #[test]
    fn test_mouse_move_round_trip() {
        let msg = KvmMessage::MouseMove(MouseMoveMessage {
            x: 1920,
            y: 1080,
            delta_x: -5,
            delta_y: 10,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_mouse_move_negative_coordinates() {
        let msg = KvmMessage::MouseMove(MouseMoveMessage {
            x: -100,
            y: -200,
            delta_x: -30,
            delta_y: -10,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    // ── MouseButton ───────────────────────────────────────────────────────────

    #[test]
    fn test_mouse_button_left_click_round_trip() {
        let msg = KvmMessage::MouseButton(MouseButtonMessage {
            button: MouseButton::Left,
            event_type: ButtonEventType::Press,
            x: 640,
            y: 480,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_mouse_button_all_buttons_round_trip() {
        for button in [
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::Button4,
            MouseButton::Button5,
        ] {
            let msg = KvmMessage::MouseButton(MouseButtonMessage {
                button,
                event_type: ButtonEventType::Release,
                x: 0,
                y: 0,
            });
            assert_eq!(round_trip(&msg), msg);
        }
    }

    // ── MouseScroll ───────────────────────────────────────────────────────────

    #[test]
    fn test_mouse_scroll_vertical_round_trip() {
        let msg = KvmMessage::MouseScroll(MouseScrollMessage {
            delta_x: 0,
            delta_y: 120,
            x: 500,
            y: 500,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_mouse_scroll_horizontal_round_trip() {
        let msg = KvmMessage::MouseScroll(MouseScrollMessage {
            delta_x: -120,
            delta_y: 0,
            x: 200,
            y: 300,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    // ── InputBatch ────────────────────────────────────────────────────────────

    #[test]
    fn test_input_batch_empty_round_trip() {
        let msg = KvmMessage::InputBatch(vec![]);
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_input_batch_mixed_events_round_trip() {
        let msg = KvmMessage::InputBatch(vec![
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
        assert_eq!(round_trip(&msg), msg);
    }

    // ── Announce / AnnounceResponse ───────────────────────────────────────────

    #[test]
    fn test_announce_round_trip() {
        let msg = KvmMessage::Announce(AnnounceMessage {
            client_id: Uuid::new_v4(),
            platform_id: PlatformId::MacOs,
            control_port: 24800,
            client_name: "macbook-pro".to_string(),
        });
        assert_eq!(round_trip(&msg), msg);
    }

    #[test]
    fn test_announce_response_round_trip() {
        let msg = KvmMessage::AnnounceResponse(AnnounceResponseMessage {
            master_control_port: 24800,
            already_paired: true,
        });
        assert_eq!(round_trip(&msg), msg);
    }

    // ── Error conditions ──────────────────────────────────────────────────────

    #[test]
    fn test_decode_empty_bytes_returns_insufficient_data() {
        let result = decode_message(&[]);
        assert!(matches!(
            result,
            Err(ProtocolError::InsufficientData { .. })
        ));
    }

    #[test]
    fn test_decode_truncated_header_returns_insufficient_data() {
        let result = decode_message(&[0x01, 0x07]); // only 2 bytes
        assert!(matches!(
            result,
            Err(ProtocolError::InsufficientData { .. })
        ));
    }

    #[test]
    fn test_decode_unknown_message_type_returns_error() {
        let mut bytes = vec![0u8; 24];
        bytes[0] = PROTOCOL_VERSION;
        bytes[1] = 0xFF; // unknown type
                         // payload_length = 0, so no payload needed
        let result = decode_message(&bytes);
        assert!(matches!(
            result,
            Err(ProtocolError::UnknownMessageType(0xFF))
        ));
    }

    #[test]
    fn test_decode_wrong_version_returns_error() {
        let mut bytes = vec![0u8; 24];
        bytes[0] = 0x99; // wrong version
        bytes[1] = MessageType::Ping as u8;
        let result = decode_message(&bytes);
        assert!(matches!(
            result,
            Err(ProtocolError::UnsupportedVersion(0x99))
        ));
    }

    #[test]
    fn test_decode_payload_length_exceeds_available_returns_error() {
        let mut bytes = vec![0u8; 24];
        bytes[0] = PROTOCOL_VERSION;
        bytes[1] = MessageType::Ping as u8;
        // Declare 100 bytes of payload, but provide none
        bytes[4..8].copy_from_slice(&100u32.to_be_bytes());
        let result = decode_message(&bytes);
        assert!(matches!(
            result,
            Err(ProtocolError::PayloadLengthMismatch { .. })
        ));
    }

    #[test]
    fn test_header_has_correct_version_byte() {
        let msg = KvmMessage::Ping(0);
        let bytes = encode_message(&msg, 1, 0).unwrap();
        assert_eq!(bytes[0], PROTOCOL_VERSION);
    }

    #[test]
    fn test_header_encodes_sequence_number_correctly() {
        let seq = 0x1234_5678_9ABC_DEF0u64;
        let bytes = encode_message(&KvmMessage::Ping(0), seq, 0).unwrap();
        let decoded_seq = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        assert_eq!(decoded_seq, seq);
    }

    #[test]
    fn test_header_encodes_timestamp_correctly() {
        let ts = 0xABCD_EF01_2345_6789u64;
        let bytes = encode_message(&KvmMessage::Ping(0), 0, ts).unwrap();
        let decoded_ts = u64::from_be_bytes(bytes[16..24].try_into().unwrap());
        assert_eq!(decoded_ts, ts);
    }

    #[test]
    fn test_header_size_is_24_bytes() {
        let msg = KvmMessage::ScreenInfoAck;
        let bytes = encode_message(&msg, 0, 0).unwrap();
        // ScreenInfoAck has empty payload so total = HEADER_SIZE
        assert_eq!(bytes.len(), HEADER_SIZE);
    }
}
