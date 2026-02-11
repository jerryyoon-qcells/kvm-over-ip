//! All KVM-Over-IP protocol message types.
//!
//! Messages follow the wire format defined in the Network Protocol Specification v1.0.
//! The canonical key representation is USB HID Usage IDs (page 0x07).

use crate::keymap::hid::HidKeyCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Protocol constants ────────────────────────────────────────────────────────

/// Current protocol version byte.
pub const PROTOCOL_VERSION: u8 = 0x01;

/// Total size of the common message header in bytes.
pub const HEADER_SIZE: usize = 24;

// ── Platform identifiers ──────────────────────────────────────────────────────

/// Platform identifier byte, included in HELLO and ANNOUNCE messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PlatformId {
    Windows = 0x01,
    Linux = 0x02,
    MacOs = 0x03,
    Web = 0x04,
}

impl TryFrom<u8> for PlatformId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(PlatformId::Windows),
            0x02 => Ok(PlatformId::Linux),
            0x03 => Ok(PlatformId::MacOs),
            0x04 => Ok(PlatformId::Web),
            _ => Err(()),
        }
    }
}

// ── Message type codes ────────────────────────────────────────────────────────

/// All message type codes defined in the protocol specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    // Control channel (0x00–0x3F)
    Hello = 0x01,
    HelloAck = 0x02,
    PairingRequest = 0x03,
    PairingResponse = 0x04,
    ScreenInfo = 0x05,
    ScreenInfoAck = 0x06,
    Ping = 0x07,
    Pong = 0x08,
    Disconnect = 0x09,
    Error = 0x0A,
    ClipboardData = 0x0B,
    ConfigUpdate = 0x0C,
    // Input channel (0x40–0x7F)
    KeyEvent = 0x40,
    MouseMove = 0x41,
    MouseButton = 0x42,
    MouseScroll = 0x43,
    InputBatch = 0x44,
    // Discovery (0x80–0x8F)
    Announce = 0x80,
    AnnounceResponse = 0x81,
}

impl TryFrom<u8> for MessageType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0x01 => Ok(MessageType::Hello),
            0x02 => Ok(MessageType::HelloAck),
            0x03 => Ok(MessageType::PairingRequest),
            0x04 => Ok(MessageType::PairingResponse),
            0x05 => Ok(MessageType::ScreenInfo),
            0x06 => Ok(MessageType::ScreenInfoAck),
            0x07 => Ok(MessageType::Ping),
            0x08 => Ok(MessageType::Pong),
            0x09 => Ok(MessageType::Disconnect),
            0x0A => Ok(MessageType::Error),
            0x0B => Ok(MessageType::ClipboardData),
            0x0C => Ok(MessageType::ConfigUpdate),
            0x40 => Ok(MessageType::KeyEvent),
            0x41 => Ok(MessageType::MouseMove),
            0x42 => Ok(MessageType::MouseButton),
            0x43 => Ok(MessageType::MouseScroll),
            0x44 => Ok(MessageType::InputBatch),
            0x80 => Ok(MessageType::Announce),
            0x81 => Ok(MessageType::AnnounceResponse),
            _ => Err(()),
        }
    }
}

// ── Common message header ─────────────────────────────────────────────────────

/// 24-byte header prepended to every message on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageHeader {
    /// Protocol version; always [`PROTOCOL_VERSION`].
    pub version: u8,
    /// Identifies the payload type.
    pub message_type: MessageType,
    /// Length of the payload in bytes (not including this header).
    pub payload_length: u32,
    /// Monotonically increasing per-channel counter.
    pub sequence_number: u64,
    /// Microseconds since Unix epoch at time of generation.
    pub timestamp_us: u64,
}

// ── Per-message payload structs ───────────────────────────────────────────────

/// HELLO (0x01): sent by client to initiate connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloMessage {
    /// UUID v4 uniquely identifying this client instance.
    pub client_id: Uuid,
    /// Protocol version the client supports.
    pub protocol_version: u8,
    /// Operating system of the client.
    pub platform_id: PlatformId,
    /// Human-readable hostname or display name.
    pub client_name: String,
    /// Bitmask of optional capabilities (keyboard, mouse, clipboard, multi-monitor).
    pub capabilities: u32,
}

/// Capability bitmask flags used in [`HelloMessage::capabilities`].
pub mod capabilities {
    pub const KEYBOARD_EMULATION: u32 = 1 << 0;
    pub const MOUSE_EMULATION: u32 = 1 << 1;
    pub const CLIPBOARD_SHARING: u32 = 1 << 2;
    pub const MULTI_MONITOR: u32 = 1 << 3;
}

/// HELLO_ACK (0x02): master response to a HELLO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloAckMessage {
    /// 32-byte random session token used to bind the DTLS input channel.
    pub session_token: [u8; 32],
    /// Protocol version the master is using.
    pub server_version: u8,
    /// Whether the connection was accepted.
    pub accepted: bool,
    /// Reason code when rejected (0x00 if accepted).
    pub reject_reason: u8,
}

/// PAIRING_REQUEST (0x03): master initiates PIN pairing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingRequestMessage {
    /// Unique identifier for this pairing session.
    pub pairing_session_id: Uuid,
    /// Expiry timestamp in seconds since Unix epoch.
    pub expires_at_secs: u64,
}

/// PAIRING_RESPONSE (0x04): client accepts/rejects a pairing request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingResponseMessage {
    /// Pairing session being responded to.
    pub pairing_session_id: Uuid,
    /// SHA-256 hash of (PIN + pairing_session_id) as hex string.
    pub pin_hash: String,
    /// Whether the client accepted the pairing request.
    pub accepted: bool,
}

/// Single monitor information within [`ScreenInfoMessage`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// Zero-based monitor index.
    pub monitor_id: u8,
    /// X offset relative to the primary monitor (may be negative).
    pub x_offset: i32,
    /// Y offset relative to the primary monitor (may be negative).
    pub y_offset: i32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// DPI scale factor times 100 (e.g., 150 = 150 %).
    pub scale_factor: u16,
    /// Whether this is the primary monitor.
    pub is_primary: bool,
}

/// SCREEN_INFO (0x05): client reports its monitor configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenInfoMessage {
    /// All active monitors on the client.
    pub monitors: Vec<MonitorInfo>,
}

/// Keyboard event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum KeyEventType {
    KeyDown = 0x01,
    KeyUp = 0x02,
}

impl TryFrom<u8> for KeyEventType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(KeyEventType::KeyDown),
            0x02 => Ok(KeyEventType::KeyUp),
            _ => Err(()),
        }
    }
}

/// Modifier key bitmask used in [`KeyEventMessage`].
///
/// Bit layout:
/// - Bit 0: Left Ctrl
/// - Bit 1: Right Ctrl
/// - Bit 2: Left Shift
/// - Bit 3: Right Shift
/// - Bit 4: Left Alt
/// - Bit 5: Right Alt
/// - Bit 6: Left Meta (Windows/Command/Super)
/// - Bit 7: Right Meta
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ModifierFlags(pub u8);

impl ModifierFlags {
    pub const LEFT_CTRL: u8 = 1 << 0;
    pub const RIGHT_CTRL: u8 = 1 << 1;
    pub const LEFT_SHIFT: u8 = 1 << 2;
    pub const RIGHT_SHIFT: u8 = 1 << 3;
    pub const LEFT_ALT: u8 = 1 << 4;
    pub const RIGHT_ALT: u8 = 1 << 5;
    pub const LEFT_META: u8 = 1 << 6;
    pub const RIGHT_META: u8 = 1 << 7;

    /// Returns `true` if either Ctrl modifier is active.
    pub fn ctrl(&self) -> bool {
        self.0 & (Self::LEFT_CTRL | Self::RIGHT_CTRL) != 0
    }

    /// Returns `true` if either Shift modifier is active.
    pub fn shift(&self) -> bool {
        self.0 & (Self::LEFT_SHIFT | Self::RIGHT_SHIFT) != 0
    }

    /// Returns `true` if either Alt modifier is active.
    pub fn alt(&self) -> bool {
        self.0 & (Self::LEFT_ALT | Self::RIGHT_ALT) != 0
    }

    /// Returns `true` if either Meta (Win/Cmd/Super) modifier is active.
    pub fn meta(&self) -> bool {
        self.0 & (Self::LEFT_META | Self::RIGHT_META) != 0
    }
}

/// KEY_EVENT (0x40): keyboard press or release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEventMessage {
    /// USB HID Usage ID (page 0x07) – platform-independent key representation.
    pub key_code: HidKeyCode,
    /// Platform scan code (informational only; not used for emulation).
    pub scan_code: u16,
    /// Whether this is a key-down or key-up event.
    pub event_type: KeyEventType,
    /// Active modifier keys at the time of the event.
    pub modifiers: ModifierFlags,
}

/// MOUSE_MOVE (0x41): absolute cursor position in client's local coordinate space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MouseMoveMessage {
    /// Absolute X position (pixels, origin at top-left of primary monitor).
    pub x: i32,
    /// Absolute Y position (pixels, origin at top-left of primary monitor).
    pub y: i32,
    /// Relative X movement delta (signed).
    pub delta_x: i16,
    /// Relative Y movement delta (signed).
    pub delta_y: i16,
}

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MouseButton {
    Left = 0x01,
    Right = 0x02,
    Middle = 0x03,
    Button4 = 0x04,
    Button5 = 0x05,
}

impl TryFrom<u8> for MouseButton {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(MouseButton::Left),
            0x02 => Ok(MouseButton::Right),
            0x03 => Ok(MouseButton::Middle),
            0x04 => Ok(MouseButton::Button4),
            0x05 => Ok(MouseButton::Button5),
            _ => Err(()),
        }
    }
}

/// Mouse button event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ButtonEventType {
    Press = 0x01,
    Release = 0x02,
}

impl TryFrom<u8> for ButtonEventType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(ButtonEventType::Press),
            0x02 => Ok(ButtonEventType::Release),
            _ => Err(()),
        }
    }
}

/// MOUSE_BUTTON (0x42): mouse button press or release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MouseButtonMessage {
    /// Which mouse button changed.
    pub button: MouseButton,
    /// Press or release.
    pub event_type: ButtonEventType,
    /// Absolute X position at time of click.
    pub x: i32,
    /// Absolute Y position at time of click.
    pub y: i32,
}

/// MOUSE_SCROLL (0x43): mouse wheel scroll event.
///
/// Scroll units: 1 unit = 1/120th of a notch (matches Windows WHEEL_DELTA convention).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MouseScrollMessage {
    /// Horizontal scroll amount (signed; positive = right).
    pub delta_x: i16,
    /// Vertical scroll amount (signed; positive = up/away from user).
    pub delta_y: i16,
    /// Absolute X position of the cursor.
    pub x: i32,
    /// Absolute Y position of the cursor.
    pub y: i32,
}

/// A single input event within an [`InputBatch`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputEvent {
    Key(KeyEventMessage),
    MouseMove(MouseMoveMessage),
    MouseButton(MouseButtonMessage),
    MouseScroll(MouseScrollMessage),
}

/// ANNOUNCE (0x80): client broadcasts its presence for discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnounceMessage {
    /// UUID of the client instance.
    pub client_id: Uuid,
    /// Operating system of the client.
    pub platform_id: PlatformId,
    /// TCP control port the client is listening on.
    pub control_port: u16,
    /// Human-readable hostname or display name.
    pub client_name: String,
}

/// ANNOUNCE_RESPONSE (0x81): master responds to a discovery broadcast.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnounceResponseMessage {
    /// Control port of the master.
    pub master_control_port: u16,
    /// Whether this client is already paired with the master.
    pub already_paired: bool,
}

/// Clipboard data format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ClipboardFormat {
    Utf8Text = 0x01,
    Html = 0x02,
    Image = 0x03,
}

impl TryFrom<u8> for ClipboardFormat {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(ClipboardFormat::Utf8Text),
            0x02 => Ok(ClipboardFormat::Html),
            0x03 => Ok(ClipboardFormat::Image),
            _ => Err(()),
        }
    }
}

/// CLIPBOARD_DATA (0x0B): clipboard content transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardDataMessage {
    /// Format of the clipboard content.
    pub format: ClipboardFormat,
    /// Raw clipboard bytes.
    pub data: Vec<u8>,
    /// True if more fragments follow (for content > 64 KB).
    pub has_more_fragments: bool,
}

/// Reason for a graceful disconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DisconnectReason {
    UserInitiated = 0x01,
    ServerShutdown = 0x02,
    ProtocolError = 0x03,
    Timeout = 0x04,
}

impl TryFrom<u8> for DisconnectReason {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(DisconnectReason::UserInitiated),
            0x02 => Ok(DisconnectReason::ServerShutdown),
            0x03 => Ok(DisconnectReason::ProtocolError),
            0x04 => Ok(DisconnectReason::Timeout),
            _ => Err(()),
        }
    }
}

/// Protocol-level error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ProtocolErrorCode {
    ProtocolVersionMismatch = 0x01,
    AuthenticationFailed = 0x02,
    PairingRequired = 0x03,
    PairingFailed = 0x04,
    TooManyClients = 0x05,
    RateLimited = 0x06,
    InternalError = 0x07,
    InvalidMessage = 0x08,
}

/// ERROR (0x0A): error notification from either side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorMessage {
    /// Protocol error code.
    pub error_code: ProtocolErrorCode,
    /// Human-readable description (not for end-user display; for logging only).
    pub description: String,
}

// ── Top-level message enum ────────────────────────────────────────────────────

/// All valid KVM-Over-IP messages, discriminated by type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KvmMessage {
    Hello(HelloMessage),
    HelloAck(HelloAckMessage),
    PairingRequest(PairingRequestMessage),
    PairingResponse(PairingResponseMessage),
    ScreenInfo(ScreenInfoMessage),
    ScreenInfoAck,
    Ping(u64),
    Pong(u64),
    Disconnect { reason: DisconnectReason },
    Error(ErrorMessage),
    ClipboardData(ClipboardDataMessage),
    KeyEvent(KeyEventMessage),
    MouseMove(MouseMoveMessage),
    MouseButton(MouseButtonMessage),
    MouseScroll(MouseScrollMessage),
    InputBatch(Vec<InputEvent>),
    Announce(AnnounceMessage),
    AnnounceResponse(AnnounceResponseMessage),
}

impl KvmMessage {
    /// Returns the [`MessageType`] discriminant for this message.
    pub fn message_type(&self) -> MessageType {
        match self {
            KvmMessage::Hello(_) => MessageType::Hello,
            KvmMessage::HelloAck(_) => MessageType::HelloAck,
            KvmMessage::PairingRequest(_) => MessageType::PairingRequest,
            KvmMessage::PairingResponse(_) => MessageType::PairingResponse,
            KvmMessage::ScreenInfo(_) => MessageType::ScreenInfo,
            KvmMessage::ScreenInfoAck => MessageType::ScreenInfoAck,
            KvmMessage::Ping(_) => MessageType::Ping,
            KvmMessage::Pong(_) => MessageType::Pong,
            KvmMessage::Disconnect { .. } => MessageType::Disconnect,
            KvmMessage::Error(_) => MessageType::Error,
            KvmMessage::ClipboardData(_) => MessageType::ClipboardData,
            KvmMessage::KeyEvent(_) => MessageType::KeyEvent,
            KvmMessage::MouseMove(_) => MessageType::MouseMove,
            KvmMessage::MouseButton(_) => MessageType::MouseButton,
            KvmMessage::MouseScroll(_) => MessageType::MouseScroll,
            KvmMessage::InputBatch(_) => MessageType::InputBatch,
            KvmMessage::Announce(_) => MessageType::Announce,
            KvmMessage::AnnounceResponse(_) => MessageType::AnnounceResponse,
        }
    }
}
