//! All KVM-Over-IP protocol message types.
//!
//! Messages follow the wire format defined in the Network Protocol Specification v1.0.
//! The canonical key representation is USB HID Usage IDs (page 0x07).
//!
//! # How messages are organised (for beginners)
//!
//! There are three categories of messages, identified by their type code range:
//!
//! | Range       | Purpose                                       |
//! |-------------|-----------------------------------------------|
//! | 0x00–0x3F   | **Control channel** – pairing, handshake, etc.|
//! | 0x40–0x7F   | **Input channel** – keyboard and mouse events.|
//! | 0x80–0x8F   | **Discovery** – finding masters on the LAN.   |
//!
//! Every message is represented in Rust as a variant of [`KvmMessage`].
//! Use [`KvmMessage::message_type`] to get the numeric type code of any message.

use crate::keymap::hid::HidKeyCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Protocol constants ────────────────────────────────────────────────────────

/// Current protocol version byte.
///
/// This value is written into byte 0 of every message header.  If a receiver
/// sees a different value it will reject the message with
/// [`crate::protocol::codec::ProtocolError::UnsupportedVersion`].
pub const PROTOCOL_VERSION: u8 = 0x01;

/// Total size of the common message header in bytes.
///
/// Every message sent over the wire starts with exactly this many bytes before
/// the variable-length payload begins.  The header layout is:
///
/// ```text
/// Offset  Size  Field
/// 0       1     version          (always PROTOCOL_VERSION = 0x01)
/// 1       1     message_type     (discriminates the payload)
/// 2       2     reserved         (must be 0x00 0x00)
/// 4       4     payload_length   (big-endian u32)
/// 8       8     sequence_number  (big-endian u64, per-channel counter)
/// 16      8     timestamp_us     (big-endian u64, microseconds since Unix epoch)
/// ```
pub const HEADER_SIZE: usize = 24;

// ── Platform identifiers ──────────────────────────────────────────────────────

/// Platform identifier byte, included in HELLO and ANNOUNCE messages.
///
/// The client sends this value so the master knows which platform-specific
/// input emulation it should use when forwarding events to that client.
/// For example, a Linux client needs X11 KeySym codes, while a Windows client
/// needs Virtual Key codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)] // Store as a single byte on the wire.
pub enum PlatformId {
    Windows = 0x01,
    Linux = 0x02,
    MacOs = 0x03,
    /// A browser-based client (e.g., the kvm-web-bridge).
    Web = 0x04,
}

impl TryFrom<u8> for PlatformId {
    type Error = ();

    /// Converts a raw byte from the wire into a [`PlatformId`].
    ///
    /// Returns `Err(())` for any value that is not a recognised platform code.
    /// The codec will convert this `Err` into a [`crate::protocol::codec::ProtocolError`].
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
///
/// The numeric value of each variant is stored in byte 1 of the header.
/// The codec uses this value to decide which decoder to call.
///
/// # Adding a new message type (for contributors)
///
/// 1. Add a variant here with its assigned code.
/// 2. Add the corresponding `TryFrom<u8>` arm below.
/// 3. Create a payload struct (if needed) in this file.
/// 4. Add an encode helper in `codec.rs → encode_payload`.
/// 5. Add a decode helper in `codec.rs → decode_payload`.
/// 6. Add a variant to [`KvmMessage`] and update `message_type()`.
/// 7. Write round-trip tests in `codec.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    // ── Control channel (0x00–0x3F) ──────────────────────────────────────────
    /// Client introduces itself and requests a connection.
    Hello = 0x01,
    /// Master accepts or rejects the Hello.
    HelloAck = 0x02,
    /// Master offers a PIN-based pairing session.
    PairingRequest = 0x03,
    /// Client submits its PIN hash to complete pairing.
    PairingResponse = 0x04,
    /// Client reports its monitor configuration.
    ScreenInfo = 0x05,
    /// Master acknowledges receipt of ScreenInfo.
    ScreenInfoAck = 0x06,
    /// Keepalive request (carries an echo token).
    Ping = 0x07,
    /// Keepalive reply (echoes back the token from Ping).
    Pong = 0x08,
    /// Either side signals a graceful disconnect.
    Disconnect = 0x09,
    /// Either side signals a protocol-level error.
    Error = 0x0A,
    /// Clipboard content transfer.
    ClipboardData = 0x0B,
    /// Master pushes live configuration changes to the client.
    ConfigUpdate = 0x0C,
    // ── Input channel (0x40–0x7F) ─────────────────────────────────────────────
    /// A single keyboard key press or release.
    KeyEvent = 0x40,
    /// Absolute cursor position update.
    MouseMove = 0x41,
    /// Mouse button press or release.
    MouseButton = 0x42,
    /// Mouse wheel scroll.
    MouseScroll = 0x43,
    /// Multiple input events bundled together for efficiency.
    InputBatch = 0x44,
    // ── Discovery (0x80–0x8F) ─────────────────────────────────────────────────
    /// Client broadcasts its presence on the LAN.
    Announce = 0x80,
    /// Master responds to an Announce broadcast.
    AnnounceResponse = 0x81,
}

impl TryFrom<u8> for MessageType {
    type Error = ();

    /// Converts a raw byte read from the message header into a [`MessageType`].
    ///
    /// Returns `Err(())` for any byte that is not a recognised type code.
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
///
/// You normally don't construct this directly; the `encode_message` function
/// builds it for you from the message content and the parameters you supply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageHeader {
    /// Protocol version; always [`PROTOCOL_VERSION`].
    pub version: u8,
    /// Identifies the payload type.
    pub message_type: MessageType,
    /// Length of the payload in bytes (not including this header).
    pub payload_length: u32,
    /// Monotonically increasing per-channel counter.
    ///
    /// Used to detect out-of-order or duplicate messages.  Starts at 0 and
    /// increments by 1 for each message sent on a given channel.
    pub sequence_number: u64,
    /// Microseconds since Unix epoch at time of generation.
    ///
    /// Useful for measuring one-way latency when the clocks of both machines
    /// are synchronised (e.g., via NTP).
    pub timestamp_us: u64,
}

// ── Per-message payload structs ───────────────────────────────────────────────

/// HELLO (0x01): sent by client to initiate connection.
///
/// This is the first message a client sends after establishing a TCP connection
/// to the master.  It is analogous to an HTTP request's `Host` header: it tells
/// the master who is calling and what they are capable of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloMessage {
    /// UUID v4 uniquely identifying this client instance.
    ///
    /// Generated once when the client application is first installed and stored
    /// persistently.  The master uses this to recognise returning clients.
    pub client_id: Uuid,
    /// Protocol version the client supports.
    pub protocol_version: u8,
    /// Operating system of the client.
    pub platform_id: PlatformId,
    /// Human-readable hostname or display name.
    ///
    /// Shown in the master's UI so the user can identify the client.
    pub client_name: String,
    /// Bitmask of optional capabilities (keyboard, mouse, clipboard, multi-monitor).
    ///
    /// Each bit signals that the client supports a particular feature.
    /// See the [`capabilities`] module for the individual flag constants.
    pub capabilities: u32,
}

/// Capability bitmask flags used in [`HelloMessage::capabilities`].
///
/// These constants define individual bits in the 32-bit `capabilities` field.
/// A client sets each bit to 1 to advertise that it supports the corresponding
/// feature.
///
/// # Example
///
/// ```
/// use kvm_core::protocol::messages::capabilities;
///
/// // A client that supports keyboard and mouse but not clipboard:
/// let caps: u32 = capabilities::KEYBOARD_EMULATION | capabilities::MOUSE_EMULATION;
/// assert!(caps & capabilities::KEYBOARD_EMULATION != 0);
/// assert!(caps & capabilities::CLIPBOARD_SHARING == 0);
/// ```
pub mod capabilities {
    /// Bit 0: client can receive and emulate keyboard events.
    pub const KEYBOARD_EMULATION: u32 = 1 << 0;
    /// Bit 1: client can receive and emulate mouse events.
    pub const MOUSE_EMULATION: u32 = 1 << 1;
    /// Bit 2: client supports clipboard content sharing.
    pub const CLIPBOARD_SHARING: u32 = 1 << 2;
    /// Bit 3: client has more than one monitor.
    pub const MULTI_MONITOR: u32 = 1 << 3;
}

/// HELLO_ACK (0x02): master response to a HELLO.
///
/// The master sends this immediately after receiving a valid `HelloMessage`.
/// If `accepted` is `false`, the client should check `reject_reason` to
/// understand why and display an appropriate error to the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloAckMessage {
    /// 32-byte random session token used to bind the DTLS input channel.
    ///
    /// The client must present this token when opening the secondary (input)
    /// channel so the master knows which TCP session it belongs to.
    pub session_token: [u8; 32],
    /// Protocol version the master is using.
    pub server_version: u8,
    /// Whether the connection was accepted.
    pub accepted: bool,
    /// Reason code when rejected (0x00 if accepted).
    ///
    /// Non-zero values correspond to specific rejection reasons (e.g., too many
    /// clients connected, or pairing required).
    pub reject_reason: u8,
}

/// PAIRING_REQUEST (0x03): master initiates PIN pairing.
///
/// When a client connects for the first time (no prior pairing relationship),
/// the master generates a 6-digit PIN, shows it to the master-side user, and
/// sends this message to prompt the client to enter the PIN.
///
/// # How PIN pairing works
///
/// 1. Master generates a random 6-digit PIN and displays it on screen.
/// 2. Master sends `PairingRequest` to the client (with a session UUID and expiry).
/// 3. Client user types the PIN into the client UI.
/// 4. Client sends `PairingResponse` with a hash of (PIN + session UUID).
/// 5. Master verifies the hash; if it matches, pairing is complete.
///
/// Hashing rather than sending the plain PIN prevents a network eavesdropper
/// from learning the PIN.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingRequestMessage {
    /// Unique identifier for this pairing session.
    ///
    /// Included in the hash input to prevent replay attacks: an attacker who
    /// intercepts a valid `PairingResponse` cannot reuse it for a different
    /// session because the session UUID would not match.
    pub pairing_session_id: Uuid,
    /// Expiry timestamp in seconds since Unix epoch.
    ///
    /// The client must reject this message and not display the PIN prompt if
    /// the current time is past this value.
    pub expires_at_secs: u64,
}

/// PAIRING_RESPONSE (0x04): client accepts/rejects a pairing request.
///
/// Sent by the client after the user types the PIN shown on the master.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingResponseMessage {
    /// Pairing session being responded to.
    pub pairing_session_id: Uuid,
    /// SHA-256 hash of (PIN + pairing_session_id) as hex string.
    ///
    /// The master hashes (PIN + pairing_session_id) on its side and
    /// compares — if the digests match, the PIN was correct.
    pub pin_hash: String,
    /// Whether the client accepted the pairing request.
    ///
    /// A client sets this to `false` if the user dismissed the PIN dialog
    /// without entering a PIN.
    pub accepted: bool,
}

/// Single monitor information within [`ScreenInfoMessage`].
///
/// Describes one physical monitor attached to the client machine.
/// The `x_offset` and `y_offset` values are relative to the *client's* primary
/// monitor, not the master's virtual coordinate space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// Zero-based monitor index.
    pub monitor_id: u8,
    /// X offset relative to the primary monitor (may be negative).
    ///
    /// Negative means the monitor is to the left of the primary monitor.
    pub x_offset: i32,
    /// Y offset relative to the primary monitor (may be negative).
    pub y_offset: i32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// DPI scale factor times 100 (e.g., 150 = 150 %).
    ///
    /// Used by the master to correctly scale mouse movement when the client
    /// has a high-DPI ("Retina") display.
    pub scale_factor: u16,
    /// Whether this is the primary monitor.
    pub is_primary: bool,
}

/// SCREEN_INFO (0x05): client reports its monitor configuration.
///
/// Sent by the client right after pairing completes, and again whenever the
/// monitor configuration changes (e.g., the user connects or disconnects a
/// monitor).  The master uses this information to build the virtual layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenInfoMessage {
    /// All active monitors on the client.
    pub monitors: Vec<MonitorInfo>,
}

/// Keyboard event type.
///
/// `KeyDown` means the key was pressed; `KeyUp` means it was released.
/// Both events are always sent so the master can track modifier key state.
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
/// A modifier key is a key that changes the meaning of other keys: Shift, Ctrl,
/// Alt, and the Windows/Command/Super key (called "Meta" here).
///
/// This struct stores the current state of *all eight* modifier key positions
/// packed into a single byte.  Each bit represents one physical key:
///
/// | Bit | Modifier       |
/// |-----|----------------|
/// | 0   | Left Ctrl      |
/// | 1   | Right Ctrl     |
/// | 2   | Left Shift     |
/// | 3   | Right Shift    |
/// | 4   | Left Alt       |
/// | 5   | Right Alt      |
/// | 6   | Left Meta      |
/// | 7   | Right Meta     |
///
/// The convenience methods `ctrl()`, `shift()`, `alt()`, and `meta()` each test
/// *both* sides (left and right), because most applications don't distinguish
/// between the two.
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
        // Bitwise AND against both LEFT_CTRL and RIGHT_CTRL.
        // If any of those bits is set the result is non-zero, which Rust
        // treats as `true` after the `!= 0` comparison.
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
///
/// This message is sent when the user presses or releases any key while the
/// master has input focus.  The `key_code` field uses USB HID Usage IDs, which
/// are platform-independent.  The receiving client translates them back to the
/// native key code for its OS before synthesising the key event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEventMessage {
    /// USB HID Usage ID (page 0x07) – platform-independent key representation.
    ///
    /// Examples: `HidKeyCode::KeyA = 0x04`, `HidKeyCode::Enter = 0x28`.
    pub key_code: HidKeyCode,
    /// Platform scan code (informational only; not used for emulation).
    ///
    /// This is the raw hardware scan code from the master's keyboard.  It is
    /// included for debugging and logging but the client ignores it when
    /// synthesising the key event.
    pub scan_code: u16,
    /// Whether this is a key-down or key-up event.
    pub event_type: KeyEventType,
    /// Active modifier keys at the time of the event.
    ///
    /// The receiver uses this to ensure the correct modifier state is applied
    /// before injecting the key event into the OS.
    pub modifiers: ModifierFlags,
}

/// MOUSE_MOVE (0x41): absolute cursor position in client's local coordinate space.
///
/// Coordinates are expressed in the client screen's local pixel space (top-left
/// is 0, 0).  Both absolute and relative (delta) values are included.
/// The client uses the absolute position with `SendInput`/XTest/CoreGraphics
/// to warp the cursor to exactly the right position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MouseMoveMessage {
    /// Absolute X position (pixels, origin at top-left of primary monitor).
    pub x: i32,
    /// Absolute Y position (pixels, origin at top-left of primary monitor).
    pub y: i32,
    /// Relative X movement delta (signed).
    ///
    /// Positive = right, negative = left.
    pub delta_x: i16,
    /// Relative Y movement delta (signed).
    ///
    /// Positive = down, negative = up.
    pub delta_y: i16,
}

/// Mouse button identifier.
///
/// Left, Right, and Middle cover the three standard buttons.  Button4 and
/// Button5 correspond to the X1 and X2 "thumb" side buttons found on many mice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MouseButton {
    Left = 0x01,
    Right = 0x02,
    Middle = 0x03,
    /// X1 side button (typically "back" in browsers).
    Button4 = 0x04,
    /// X2 side button (typically "forward" in browsers).
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
///
/// `Press` means the button was pushed down; `Release` means it was let go.
/// Applications that need to detect "click" interpret a Press immediately
/// followed by a Release on the same button as a single click.
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
///
/// The cursor position at the time of the click is included so the receiving
/// client can verify (or correct) cursor placement before injecting the event.
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
///
/// # Why 1/120th of a notch?
///
/// Windows defines `WHEEL_DELTA = 120`.  One physical "click" of a scroll wheel
/// produces a delta of ±120.  By using fractions, smooth-scrolling devices can
/// report partial notches.  Most code simply checks whether the delta is positive
/// or negative and treats each 120 units as one logical scroll step.
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
///
/// An `InputBatch` message packs several individual input events into a single
/// network packet.  This reduces per-packet overhead when multiple events occur
/// within the same time window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputEvent {
    Key(KeyEventMessage),
    MouseMove(MouseMoveMessage),
    MouseButton(MouseButtonMessage),
    MouseScroll(MouseScrollMessage),
}

/// ANNOUNCE (0x80): client broadcasts its presence for discovery.
///
/// The client sends this as a UDP broadcast on the local network so that master
/// applications can discover it without the user having to manually enter an IP
/// address.
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
///
/// When the master sees an `AnnounceMessage`, it replies with this message so
/// the client knows how to connect to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnounceResponseMessage {
    /// Control port of the master.
    pub master_control_port: u16,
    /// Whether this client is already paired with the master.
    ///
    /// If `true`, the client can connect immediately without a PIN prompt.
    pub already_paired: bool,
}

/// Clipboard data format.
///
/// Indicates how the bytes in [`ClipboardDataMessage::data`] should be
/// interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ClipboardFormat {
    /// Plain UTF-8 text (most common).
    Utf8Text = 0x01,
    /// HTML markup.
    Html = 0x02,
    /// Image data (format-specific; typically PNG).
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
///
/// Large clipboard contents (e.g., images) are split into multiple fragments.
/// Check `has_more_fragments`; if `true`, wait for more `ClipboardData` messages
/// with the same format before reassembling.
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
///
/// Either side can send a `Disconnect` message before closing the TCP
/// connection.  The `reason` field tells the peer *why* the connection is
/// being closed so it can display an appropriate message to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DisconnectReason {
    /// The user explicitly closed the connection (e.g., clicked "Disconnect").
    UserInitiated = 0x01,
    /// The master application is shutting down.
    ServerShutdown = 0x02,
    /// A protocol-level error occurred that cannot be recovered from.
    ProtocolError = 0x03,
    /// The keepalive timeout expired (no Ping/Pong for too long).
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
///
/// These accompany an [`ErrorMessage`] and tell the receiver what went wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ProtocolErrorCode {
    /// The client's protocol version is incompatible with the master's.
    ProtocolVersionMismatch = 0x01,
    /// The pairing PIN hash did not match.
    AuthenticationFailed = 0x02,
    /// This client has not yet completed pairing.
    PairingRequired = 0x03,
    /// Pairing failed (e.g., PIN expired or incorrect).
    PairingFailed = 0x04,
    /// The master has reached its maximum client limit.
    TooManyClients = 0x05,
    /// The client is sending messages too quickly.
    RateLimited = 0x06,
    /// An unexpected internal error occurred on the remote side.
    InternalError = 0x07,
    /// The received message could not be decoded (malformed bytes).
    InvalidMessage = 0x08,
}

/// ERROR (0x0A): error notification from either side.
///
/// When an unrecoverable protocol-level error occurs, the sending side
/// transmits this message and then closes the connection.  The `description`
/// field is for logging only — it should not be shown directly to end users.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorMessage {
    /// Protocol error code.
    pub error_code: ProtocolErrorCode,
    /// Human-readable description (not for end-user display; for logging only).
    pub description: String,
}

/// Bitmask flags carried in [`ConfigUpdateMessage::flags`].
///
/// Each bit represents a boolean setting that the master wants the client to
/// know about.  Bits not listed here are reserved and must be set to zero.
///
/// # How bitmasks work (for beginners)
///
/// A bitmask stores many true/false values packed into a single integer.  Each
/// "flag" is one bit.  To test whether a flag is set you use bitwise AND (`&`):
///
/// ```
/// use kvm_core::protocol::messages::config_flags;
/// let flags: u32 = config_flags::AUTOSTART;
/// assert!(flags & config_flags::AUTOSTART != 0); // autostart is ON
/// ```
pub mod config_flags {
    /// Bit 0: whether the master starts automatically on OS login.
    pub const AUTOSTART: u32 = 1 << 0;
}

/// CONFIG_UPDATE (0x0C): master pushes live configuration changes to a connected client.
///
/// This message is sent by the master whenever the user changes a setting that
/// affects how the client should behave.  The client applies the new values
/// without requiring a reconnect.
///
/// # Wire layout (big-endian)
///
/// ```text
/// [log_level_len : 2 bytes][log_level     : N bytes]
/// [hotkey_len    : 2 bytes][disable_hotkey : M bytes]
/// [flags         : 4 bytes]
/// ```
///
/// # Bug fix note (Bug 2)
///
/// This variant was previously missing from [`KvmMessage`] and the codec
/// incorrectly returned `KvmMessage::ScreenInfoAck` for 0x0C messages.
/// It is now correctly defined and the codec handles it properly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigUpdateMessage {
    /// Desired `tracing` log level for the remote side.
    /// Valid values: `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"`.
    pub log_level: String,

    /// Human-readable description of the hotkey that switches input focus away
    /// from the client back to the master (e.g. `"ScrollLock+ScrollLock"`).
    /// The client can display this in its UI so the user knows how to escape.
    pub disable_hotkey: String,

    /// Packed boolean settings — see [`config_flags`] for the individual bits.
    pub flags: u32,
}

// ── Top-level message enum ────────────────────────────────────────────────────

/// All valid KVM-Over-IP messages, discriminated by type.
///
/// Each variant wraps a specific payload struct.  The codec in
/// `kvm_core::protocol::codec` knows how to turn every variant into bytes and
/// back again.
///
/// # Usage example
///
/// ```rust
/// use kvm_core::protocol::messages::{KvmMessage, KeyEventMessage, KeyEventType, ModifierFlags};
/// use kvm_core::keymap::hid::HidKeyCode;
///
/// // Build a message representing the user pressing the letter A.
/// let msg = KvmMessage::KeyEvent(KeyEventMessage {
///     key_code: HidKeyCode::KeyA,
///     scan_code: 0x001E,
///     event_type: KeyEventType::KeyDown,
///     modifiers: ModifierFlags::default(), // no modifiers held
/// });
///
/// // Access the numeric type discriminant.
/// use kvm_core::protocol::messages::MessageType;
/// assert_eq!(msg.message_type(), MessageType::KeyEvent);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KvmMessage {
    Hello(HelloMessage),
    HelloAck(HelloAckMessage),
    PairingRequest(PairingRequestMessage),
    PairingResponse(PairingResponseMessage),
    ScreenInfo(ScreenInfoMessage),
    /// Acknowledgement that screen info was received; carries no payload.
    ScreenInfoAck,
    /// Carries a 64-bit echo token; the receiver must reply with `Pong(token)`.
    Ping(u64),
    /// Reply to a `Ping`; carries the same token that was in the Ping.
    Pong(u64),
    Disconnect {
        reason: DisconnectReason,
    },
    Error(ErrorMessage),
    ClipboardData(ClipboardDataMessage),
    /// FIX (Bug 2): this variant was previously missing from the enum even though
    /// `MessageType::ConfigUpdate` (0x0C) was defined and the decoder contained a
    /// placeholder that returned the wrong variant type (`ScreenInfoAck`).
    /// The variant is now present and the codec encodes/decodes it correctly.
    ConfigUpdate(ConfigUpdateMessage),
    KeyEvent(KeyEventMessage),
    MouseMove(MouseMoveMessage),
    MouseButton(MouseButtonMessage),
    MouseScroll(MouseScrollMessage),
    /// A batch of input events packed into a single message for efficiency.
    InputBatch(Vec<InputEvent>),
    Announce(AnnounceMessage),
    AnnounceResponse(AnnounceResponseMessage),
}

impl KvmMessage {
    /// Returns the [`MessageType`] discriminant for this message.
    ///
    /// This is used by the codec to write the correct byte into position 1 of
    /// the outgoing message header.
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
            KvmMessage::ConfigUpdate(_) => MessageType::ConfigUpdate,
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
