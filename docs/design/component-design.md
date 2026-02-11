# Component Design Document
# KVM-Over-IP: Detailed Component Interfaces and Contracts

**Document Version**: 1.0
**Date**: 2026-02-10
**Status**: Approved for Development

---

## 1. kvm-core Library

### 1.1 Protocol Module

**Responsibility**: Define all message types and provide serialization/deserialization for the wire protocol.

**Public Interface** (Rust):
```rust
// crates/kvm-core/src/protocol/mod.rs

pub mod messages;
pub mod codec;

// All protocol message types
pub use messages::*;
pub use codec::{encode_message, decode_message, ProtocolError};
```

```rust
// crates/kvm-core/src/protocol/messages.rs

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MessageHeader {
    pub version: u8,
    pub message_type: MessageType,
    pub payload_length: u32,
    pub sequence_number: u64,
    pub timestamp_us: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KvmMessage {
    Hello(HelloMessage),
    HelloAck(HelloAckMessage),
    ScreenInfo(ScreenInfoMessage),
    ScreenInfoAck,
    Ping(u64),    // echo token
    Pong(u64),    // echo token
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
    PairingRequest(PairingRequestMessage),
    PairingResponse(PairingResponseMessage),
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyEventMessage {
    pub key_code: HidKeyCode,    // USB HID Usage ID
    pub scan_code: u16,
    pub event_type: KeyEventType,
    pub modifiers: ModifierFlags,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MouseMoveMessage {
    pub x: i32,
    pub y: i32,
    pub delta_x: i16,
    pub delta_y: i16,
}

// ... (all message structs)

pub fn encode_message(msg: &KvmMessage) -> Result<Vec<u8>, ProtocolError>;
pub fn decode_message(bytes: &[u8]) -> Result<(KvmMessage, usize), ProtocolError>;
```

**Unit Tests Required**:
- Round-trip encode/decode for every message variant.
- Decode with truncated bytes returns appropriate error.
- Decode with invalid message type byte returns appropriate error.
- Sequence number increments correctly across multiple encodes.
- Edge cases: max-length strings, zero-length clipboard, all modifier flags set.

### 1.2 Domain Module

**Responsibility**: Core business entities with no infrastructure dependencies.

```rust
// crates/kvm-core/src/domain/layout.rs

use std::collections::HashMap;
use uuid::Uuid;

pub type ClientId = Uuid;

#[derive(Debug, Clone)]
pub struct VirtualLayout {
    pub master: ScreenRegion,
    clients: HashMap<ClientId, ClientScreen>,
    adjacencies: Vec<Adjacency>,
}

impl VirtualLayout {
    pub fn new(master_width: u32, master_height: u32) -> Self;
    pub fn add_client(&mut self, client: ClientScreen) -> Result<(), LayoutError>;
    pub fn remove_client(&mut self, client_id: ClientId);
    pub fn update_client_region(&mut self, client_id: ClientId, region: ScreenRegion);
    pub fn set_adjacency(&mut self, adj: Adjacency) -> Result<(), LayoutError>;

    /// Given cursor position in master coordinates, returns the active screen
    /// and the cursor position in that screen's local coordinate space.
    pub fn resolve_cursor(&self, master_x: i32, master_y: i32) -> CursorLocation;

    /// Checks if the cursor is within EDGE_THRESHOLD of any transition edge.
    /// Returns Some(Transition) if a transition should occur.
    pub fn check_edge_transition(
        &self,
        current_screen: &ScreenId,
        local_x: i32,
        local_y: i32,
    ) -> Option<EdgeTransition>;

    /// Maps a position along a source edge to the proportional position on the target edge.
    pub fn map_edge_position(
        from_edge: &EdgeSegment,
        to_edge: &EdgeSegment,
        pos: i32,
    ) -> i32;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenRegion {
    pub virtual_x: i32,   // Position in virtual space
    pub virtual_y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub enum CursorLocation {
    OnMaster { local_x: i32, local_y: i32 },
    OnClient { client_id: ClientId, local_x: i32, local_y: i32 },
}

#[derive(Debug, Clone)]
pub struct EdgeTransition {
    pub to_screen: ScreenId,
    pub entry_x: i32,
    pub entry_y: i32,
    pub master_teleport_x: i32,   // Where to teleport master cursor after transition
    pub master_teleport_y: i32,
}
```

**Unit Tests Required**:
- `resolve_cursor`: cursor on master, cursor on client, cursor outside all screens.
- `check_edge_transition`: near edge (within threshold), far from edge, exactly on edge.
- `map_edge_position`: proportional mapping with same-size screens, different-size screens.
- `add_client`: adding overlapping client returns error.
- `set_adjacency`: invalid edge combinations return error.

### 1.3 Keymap Module

**Responsibility**: Static lookup tables for cross-platform key code translation.

```rust
// crates/kvm-core/src/keymap/mod.rs

pub mod hid;
pub mod windows_vk;
pub mod linux_x11;
pub mod macos_cg;

pub use hid::HidKeyCode;

pub struct KeyMapper;

impl KeyMapper {
    /// Translates Windows Virtual Key code to HID Usage ID.
    /// Returns HidKeyCode::Unknown if no mapping exists.
    pub fn windows_vk_to_hid(vk: u8) -> HidKeyCode;

    /// Translates HID Usage ID to Windows Virtual Key code.
    pub fn hid_to_windows_vk(hid: HidKeyCode) -> Option<u8>;

    /// Translates HID Usage ID to X11 KeySym (Linux).
    pub fn hid_to_x11_keysym(hid: HidKeyCode) -> Option<u32>;

    /// Translates HID Usage ID to CGKeyCode (macOS).
    pub fn hid_to_macos_cgkeycode(hid: HidKeyCode) -> Option<u16>;

    /// Translates HID Usage ID to DOM KeyboardEvent.code string (Web).
    pub fn hid_to_dom_code(hid: HidKeyCode) -> Option<&'static str>;
}
```

**Unit Tests Required**:
- All 104 standard keyboard keys in the translation table have valid mappings in both directions.
- Round-trip translation: VK -> HID -> VK returns original VK (or documented exception).
- Unknown VK codes return `HidKeyCode::Unknown`.
- Translation tables are complete (no panics on any u8 input).

---

## 2. kvm-master Components

### 2.1 InputCaptureService

**Responsibility**: Install and manage Windows low-level keyboard and mouse hooks; deliver captured events to the async runtime.

**Interface**:
```rust
// crates/kvm-master/src/infrastructure/input_capture/mod.rs

pub struct InputCaptureService {
    // Internal: hook handles, event sender
}

impl InputCaptureService {
    /// Creates and starts the capture service.
    /// Spawns a dedicated Win32 message loop thread.
    /// Returns an mpsc receiver for captured events.
    pub fn start() -> Result<(Self, mpsc::Receiver<RawInputEvent>), CaptureError>;

    /// Suppresses the current event from reaching the local system.
    /// Must be called synchronously within the hook callback window.
    /// Uses an atomic flag set by the router before the callback returns.
    pub fn suppress_current_event(&self);

    /// Stops hooks and joins the message loop thread.
    pub fn stop(self);
}

#[derive(Debug, Clone)]
pub enum RawInputEvent {
    KeyDown { vk_code: u8, scan_code: u16, time_ms: u32 },
    KeyUp   { vk_code: u8, scan_code: u16, time_ms: u32 },
    MouseMove { x: i32, y: i32, time_ms: u32 },
    MouseButtonDown { button: MouseButton, x: i32, y: i32, time_ms: u32 },
    MouseButtonUp   { button: MouseButton, x: i32, y: i32, time_ms: u32 },
    MouseWheel { delta: i16, x: i32, y: i32, time_ms: u32 },
    MouseWheelH { delta: i16, x: i32, y: i32, time_ms: u32 },
}
```

**Implementation Notes**:
- Hook callbacks use a static `AtomicBool` for the suppress flag.
- Ring buffer capacity: 4096 events (prevents hook timeout under burst input).
- Thread priority: `THREAD_PRIORITY_TIME_CRITICAL` for the hook message loop.

**Unit Tests**: Cannot directly test Windows hooks in unit tests. Use an abstraction trait `InputSource` that the hook implements; mock in tests.

### 2.2 LayoutEngine (Use Case + Domain)

**Responsibility**: Maintain the virtual layout, process cursor position updates, and determine routing targets.

```rust
// crates/kvm-master/src/application/route_input.rs

pub struct RouteInputUseCase {
    layout: VirtualLayout,      // from kvm-core
    active_target: ActiveTarget,
    cursor_pos: CursorPosition,
    transmitter: Arc<dyn InputTransmitter>,  // trait object for testability
    cursor_controller: Arc<dyn CursorController>,
}

impl RouteInputUseCase {
    pub async fn handle_event(&mut self, event: RawInputEvent) -> Result<(), RouteError>;
    pub fn update_layout(&mut self, layout: VirtualLayout);
    pub fn get_active_target(&self) -> &ActiveTarget;
}

// Testable trait for the transmitter
#[async_trait]
pub trait InputTransmitter: Send + Sync {
    async fn send_key_event(&self, client_id: ClientId, event: KeyEventMessage) -> Result<(), TransmitError>;
    async fn send_mouse_move(&self, client_id: ClientId, event: MouseMoveMessage) -> Result<(), TransmitError>;
    // ...
}

// Testable trait for cursor control
pub trait CursorController: Send + Sync {
    fn teleport_cursor(&self, x: i32, y: i32);
    fn get_cursor_pos(&self) -> (i32, i32);
}
```

**Unit Tests Required**:
- Event with active target routes to correct client.
- Event with no active target is passed through locally.
- Edge transition triggers target change and cursor teleport.
- Hotkey (disable) event stops routing and releases hook suppression.
- Mock transmitter confirms correct message type and content.

### 2.3 ConnectionManager

**Responsibility**: Accept incoming client connections, manage TLS sessions, handle pairing, and maintain the client registry.

```rust
// crates/kvm-master/src/infrastructure/network/connection_manager.rs

pub struct ConnectionManager {
    config: NetworkConfig,
    client_registry: Arc<RwLock<ClientRegistry>>,
    pairing_store: Arc<dyn PairingStore>,
    event_bus: mpsc::Sender<ConnectionEvent>,
}

impl ConnectionManager {
    pub async fn start(config: NetworkConfig) -> Result<(Self, mpsc::Receiver<ConnectionEvent>), NetworkError>;

    /// Initiates pairing with a discovered client.
    pub async fn initiate_pairing(&self, client_id: ClientId) -> Result<PairingSession, PairingError>;

    /// Confirms pairing (called when operator confirms PIN).
    pub async fn confirm_pairing(&self, session_id: Uuid, pin: &str) -> Result<(), PairingError>;

    pub fn get_connected_clients(&self) -> Vec<ConnectedClientInfo>;
}

#[derive(Debug)]
pub enum ConnectionEvent {
    ClientDiscovered { client_id: ClientId, info: AnnounceMessage },
    ClientConnected { client_id: ClientId },
    ClientDisconnected { client_id: ClientId, reason: DisconnectReason },
    PairingRequested { client_id: ClientId },
    PairingCompleted { client_id: ClientId },
    ScreenInfoUpdated { client_id: ClientId, info: ScreenInfoMessage },
}
```

---

## 3. kvm-client Components

### 3.1 InputEmulationService (Platform Abstraction)

**Responsibility**: Receive translated input events and inject them via platform APIs.

```rust
// crates/kvm-client/src/application/emulate_input.rs

// This is the use-case-level interface (platform-agnostic)
pub struct EmulateInputUseCase {
    emulator: Arc<dyn PlatformInputEmulator>,
    dedup_filter: DedupFilter,
}

impl EmulateInputUseCase {
    pub fn handle_key_event(&self, event: KeyEventMessage) -> Result<(), EmulationError>;
    pub fn handle_mouse_move(&self, event: MouseMoveMessage) -> Result<(), EmulationError>;
    pub fn handle_mouse_button(&self, event: MouseButtonMessage) -> Result<(), EmulationError>;
    pub fn handle_mouse_scroll(&self, event: MouseScrollMessage) -> Result<(), EmulationError>;
}

// Platform-specific trait
pub trait PlatformInputEmulator: Send + Sync {
    fn emit_key_down(&self, key: HidKeyCode, modifiers: ModifierFlags) -> Result<(), EmulationError>;
    fn emit_key_up(&self, key: HidKeyCode, modifiers: ModifierFlags) -> Result<(), EmulationError>;
    fn emit_mouse_move(&self, x: i32, y: i32) -> Result<(), EmulationError>;
    fn emit_mouse_button(&self, button: MouseButton, pressed: bool, x: i32, y: i32) -> Result<(), EmulationError>;
    fn emit_mouse_scroll(&self, delta_x: i16, delta_y: i16) -> Result<(), EmulationError>;
}
```

**Platform Implementations**:
```rust
// Platform-specific implementations selected at compile time

#[cfg(target_os = "windows")]
pub struct WindowsInputEmulator { /* SendInput handle */ }

#[cfg(target_os = "linux")]
pub struct LinuxXTestEmulator { display: *mut x11::xlib::Display }

#[cfg(target_os = "linux")]
pub struct LinuxUinputEmulator { fd: std::fs::File }

#[cfg(target_os = "macos")]
pub struct MacosInputEmulator { /* CGEventSource */ }
```

### 3.2 ScreenInfoService

**Responsibility**: Enumerate connected monitors and report to master; detect and report changes.

```rust
// crates/kvm-client/src/application/report_screens.rs

pub struct ScreenInfoService {
    connection: Arc<dyn ControlConnection>,
    last_reported: Option<ScreenInfoMessage>,
}

impl ScreenInfoService {
    pub async fn report_current_screens(&self) -> Result<(), ReportError>;

    /// Starts a background task that monitors for screen configuration changes
    /// and automatically reports them to the master.
    pub async fn start_monitor_watch(self: Arc<Self>) -> tokio::task::JoinHandle<()>;
}

// Platform screen enumeration (returns ScreenInfoMessage)
pub trait ScreenEnumerator: Send + Sync {
    fn enumerate_screens(&self) -> Result<ScreenInfoMessage, ScreenError>;
}

#[cfg(target_os = "windows")]
pub struct WindowsScreenEnumerator;  // Uses EnumDisplayMonitors

#[cfg(target_os = "linux")]
pub struct X11ScreenEnumerator;      // Uses XRandR

#[cfg(target_os = "macos")]
pub struct MacosScreenEnumerator;    // Uses NSScreen
```

---

## 4. Web Bridge Component

### 4.1 kvm-web-bridge

**Responsibility**: Accept WebSocket connections from web clients and proxy them to the master's native protocol.

```
  Web Browser (WSS:443 or WSS:24803)
        |
        | TLS WebSocket
        v
  kvm-web-bridge (Rust binary, runs alongside master or standalone)
        |
        | Translates: WebSocket frames <-> DTLS/TLS binary protocol
        v
  kvm-master (TLS:24800 + DTLS:24801)
```

**Key Behaviors**:
- The web bridge is a thin protocol translator; no business logic.
- It forwards AUTH handshake between browser and master.
- Translates WebSocket text/binary frames to KvmMessage binary encoding.
- Runs as a separate process to isolate web security context.

---

## 5. Tauri Command Interface (UI Bridge)

Tauri commands are the API between the React UI and the Rust backend.

### 5.1 Master Tauri Commands

```typescript
// TypeScript bindings generated by Tauri

// Get current state
invoke<LayoutState>('get_layout')
invoke<ClientInfo[]>('get_clients')
invoke<AppSettings>('get_settings')

// Layout management
invoke<void>('update_layout', { layout: LayoutState })
invoke<void>('set_client_position', { clientId: string, x: number, y: number })

// Client management
invoke<void>('initiate_pairing', { clientId: string })
invoke<void>('confirm_pairing', { sessionId: string, pin: string })
invoke<void>('disconnect_client', { clientId: string })
invoke<void>('remove_client', { clientId: string })

// Settings
invoke<void>('update_settings', { settings: Partial<AppSettings> })
invoke<void>('set_hotkey', { hotkey: string })

// Control
invoke<void>('toggle_sharing', {})
invoke<SharingState>('get_sharing_state')
```

### 5.2 Master Tauri Events (Backend -> Frontend)

```typescript
// Events emitted from Rust backend to React

listen<ClientStatusEvent>('client-status-changed')
listen<LayoutChangedEvent>('layout-changed')
listen<SharingStateEvent>('sharing-state-changed')
listen<StatsUpdateEvent>('stats-update')           // 100ms interval
listen<PairingEventData>('pairing-event')
listen<DiscoveredClientEvent>('client-discovered')
```

### 5.3 Client Tauri Commands

```typescript
invoke<ConnectionStatus>('get_status')
invoke<ClientSettings>('get_settings')
invoke<void>('update_settings', { settings: Partial<ClientSettings> })
invoke<void>('connect_to_master', { host: string, port: number })
invoke<void>('disconnect')
invoke<void>('start_discovery')
```

---

## 6. Data Models

### 6.1 Configuration Data Model

```
AppConfig
  |-- MasterConfig
  |     |-- version: String
  |     |-- autostart: bool
  |     |-- disable_hotkey: HotkeyConfig
  |     |-- log_level: LogLevel
  |     |-- network: NetworkConfig
  |     |     |-- control_port: u16
  |     |     |-- input_port: u16
  |     |     |-- discovery_port: u16
  |     |     |-- bind_address: IpAddr
  |     |-- layout: LayoutConfig
  |           |-- master_width: u32
  |           |-- master_height: u32
  |           |-- clients: Vec<ClientLayoutConfig>
  |                 |-- client_id: Uuid
  |                 |-- name: String
  |                 |-- x_offset: i32
  |                 |-- y_offset: i32
  |                 |-- width: u32
  |                 |-- height: u32
  |
  |-- ClientConfig
        |-- version: String
        |-- autostart: bool
        |-- log_level: LogLevel
        |-- network: ClientNetworkConfig
              |-- control_port: u16
              |-- input_port: u16
              |-- master_host: Option<String>
```

### 6.2 Runtime State Model

```
MasterRuntimeState
  |-- sharing_enabled: bool
  |-- active_target: Option<ClientId>
  |-- cursor_position: CursorPosition
  |-- layout: VirtualLayout
  |-- clients: HashMap<ClientId, ClientRuntimeState>
        |-- id: ClientId
        |-- name: String
        |-- address: SocketAddr
        |-- connection_state: ConnectionState
        |-- latency_ms: f32          (rolling average)
        |-- events_per_second: u32
        |-- screen_info: Option<ScreenInfoMessage>
        |-- last_heartbeat: Instant
```
