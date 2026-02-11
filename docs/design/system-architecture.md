# System Architecture Design Document
# KVM-Over-IP: Comprehensive Architecture Reference

**Document Version**: 1.0
**Date**: 2026-02-10
**Status**: Approved for Development

---

## 1. System Overview

KVM-Over-IP is a distributed, client-server application with a single Windows master node and multiple cross-platform client nodes. The system architecture emphasizes:

- **Low-latency data flow**: Input events take the shortest possible path from capture to emulation.
- **Security-first design**: All communication is encrypted and authenticated; keys never leave the master in plaintext.
- **Platform abstraction**: A shared core library isolates cross-platform concerns; platform-specific code is minimal and well-defined.
- **Clean Architecture**: Business logic is independent of UI frameworks, OS APIs, and network libraries.

---

## 2. High-Level System Architecture Diagram

```
+==============================================================+
|                    LOCAL AREA NETWORK                        |
|                                                              |
|  +----------------------+     UDP/DTLS      +-----------+   |
|  |   MASTER (Windows)   |<--- Input Events <-|  CLIENT  |   |
|  |                      |                   |  (Linux)  |   |
|  |  [Keyboard] [Mouse]  |--- Input Events ->|           |   |
|  |         |            |                   +-----------+   |
|  |  +------v--------+   |                                   |
|  |  | Input Capture |   |   TCP/TLS         +-----------+   |
|  |  | Service       |   |<-- Control Msgs ->|  CLIENT  |   |
|  |  +------+--------+   |                   | (Windows) |   |
|  |         |            |--- Input Events ->|           |   |
|  |  +------v--------+   |                   +-----------+   |
|  |  | Layout Engine |   |                                   |
|  |  | + Router      |   |   TCP/TLS         +-----------+   |
|  |  +------+--------+   |<-- Control Msgs ->|  CLIENT  |   |
|  |         |            |                   |  (macOS)  |   |
|  |  +------v--------+   |--- Input Events ->|           |   |
|  |  | Connection Mgr|   |                   +-----------+   |
|  |  | + Transmitter |   |                                   |
|  |  +------+--------+   |   WSS             +-----------+   |
|  |         |            |<-- Control Msgs ->|  CLIENT  |   |
|  |  +------v--------+   |                   | (Web Brs) |   |
|  |  |  Tauri UI     |   |--- Input Events ->|           |   |
|  |  |  (React)      |   |                   +-----------+   |
|  |  +---------------+   |                                   |
|  +----------------------+     UDP Broadcast  (Discovery)    |
|                                                              |
+==============================================================+
```

---

## 3. Master Application Architecture

### 3.1 Rust Crate Structure

```
kvm-over-ip/                          (Workspace root)
  Cargo.toml                          (Workspace manifest)
  |
  +-- crates/
  |     |
  |     +-- kvm-core/                 (Shared library)
  |     |     src/
  |     |       lib.rs
  |     |       protocol/             (Message types, codec)
  |     |       domain/               (Entities, use case interfaces)
  |     |       crypto/               (TLS/DTLS wrappers)
  |     |       keymap/               (Key code translation tables)
  |     |
  |     +-- kvm-master/               (Master binary)
  |     |     src/
  |     |       main.rs
  |     |       application/          (Use cases)
  |     |         route_input.rs
  |     |         manage_clients.rs
  |     |         update_layout.rs
  |     |       infrastructure/
  |     |         input_capture/      (Windows WH hooks)
  |     |         network/            (TLS server, DTLS sender)
  |     |         storage/            (Config file, key storage)
  |     |         ui_bridge/          (Tauri command handlers)
  |     |
  |     +-- kvm-client/               (Client binary - all platforms)
  |     |     src/
  |     |       main.rs
  |     |       application/
  |     |         emulate_input.rs
  |     |         report_screens.rs
  |     |       infrastructure/
  |     |         input_emulation/
  |     |           windows.rs        (#[cfg(target_os = "windows")])
  |     |           linux.rs          (#[cfg(target_os = "linux")])
  |     |           macos.rs          (#[cfg(target_os = "macos")])
  |     |         network/            (TLS client, DTLS receiver)
  |     |         screen_info/        (Platform screen enumeration)
  |     |         ui_bridge/
  |     |
  |     +-- kvm-web-bridge/           (WebSocket-to-DTLS bridge)
  |           src/
  |             main.rs               (Lightweight server)
  |
  +-- packages/
        |
        +-- ui-master/               (React app for master)
        |     src/
        |       components/
        |         LayoutEditor/
        |         ClientList/
        |         StatusBar/
        |       hooks/
        |       store/               (Zustand state)
        |
        +-- ui-client/               (React app for client)
              src/
                components/
                  StatusDisplay/
                  Settings/
```

### 3.2 Master Application Component Detail

```
+================================================================+
|                    kvm-master Binary                           |
+================================================================+
|                                                                |
|  INFRASTRUCTURE LAYER                                          |
|  +--------------------------+  +----------------------------+  |
|  | InputCaptureService      |  | StorageService             |  |
|  |                          |  |                            |  |
|  | Windows WH_KEYBOARD_LL   |  | ConfigRepository           |  |
|  | Windows WH_MOUSE_LL      |  | KeyringRepository          |  |
|  | Hook thread (Win32 msg   |  | (DPAPI encrypted)          |  |
|  |  loop)                   |  +----------------------------+  |
|  | Event queue (lock-free)  |                                  |
|  +--------------------------+  +----------------------------+  |
|           |                   | NetworkService             |  |
|           v                   |                            |  |
|  APPLICATION LAYER            | TcpListener (TLS 1.3)      |  |
|  +--------------------------+ | UdpSocket (DTLS 1.3)       |  |
|  | RouteInputUseCase        | | DiscoveryResponder         |  |
|  |                          | | WebSocketBridge            |  |
|  | Receives raw events      | +----------------------------+  |
|  | Queries LayoutEngine     |           |                      |
|  | Dispatches to router     |           v                      |
|  +--------------------------+  +----------------------------+  |
|           |                   | TransmissionService        |  |
|           v                   |                            |  |
|  DOMAIN LAYER                 | SequenceCounter            |  |
|  +--------------------------+ | EventSerializer            |  |
|  | LayoutEngine             | | DTLS send queue            |  |
|  |                          | +----------------------------+  |
|  | Screen virtual space     |                                  |
|  | Edge detection           |  +----------------------------+  |
|  | Route resolution         |  | UI Bridge (Tauri)          |  |
|  | Coordinate mapping       |  |                            |  |
|  +--------------------------+  | Tauri commands             |  |
|           |                   | Event emitter -> React     |  |
|  +--------------------------+  +----------------------------+  |
|  | ClientRegistry           |                                  |
|  |                          |                                  |
|  | Connected clients map    |                                  |
|  | Session state            |                                  |
|  | Pairing state            |                                  |
|  +--------------------------+                                  |
|                                                                |
+================================================================+
```

---

## 4. Client Application Architecture

### 4.1 Client Component Detail

```
+================================================================+
|                    kvm-client Binary                           |
+================================================================+
|                                                                |
|  INFRASTRUCTURE LAYER                                          |
|  +---------------------------+  +--------------------------+  |
|  | InputEmulationService     |  | NetworkService           |  |
|  |                           |  |                          |  |
|  | Platform-specific:        |  | TcpStream (TLS 1.3)      |  |
|  |  Windows: SendInput API   |  | UdpSocket (DTLS 1.3)     |  |
|  |  Linux: XTest / uinput    |  | DiscoveryBroadcaster     |  |
|  |  macOS: CGEvent API       |  | ReconnectionManager      |  |
|  |  Web: DOM Events          |  +--------------------------+  |
|  +---------------------------+            |                    |
|           ^                              v                     |
|           |                  APPLICATION LAYER                 |
|           |                  +--------------------------+      |
|  DOMAIN   |                  | HandleInputEventUseCase  |      |
|  +-------------------------------+                      |      |
|  | InputEvent (domain entity)    |  Deserialize event   |      |
|  | KeyEvent / MouseMoveEvent /   |  Validate sequence   |      |
|  | MouseButtonEvent / ScrollEvent|  Deduplicate         |      |
|  +-------------------------------+  Dispatch to emulator|      |
|                                  +--------------------------+  |
|  +---------------------------+  +--------------------------+  |
|  | ScreenInfoService         |  | SessionManager           |  |
|  |                           |  |                          |  |
|  | Enumerate monitors        |  | Auth state machine       |  |
|  | Detect changes            |  | Pairing PIN handling     |  |
|  | Report to master          |  | Session token storage    |  |
|  +---------------------------+  +--------------------------+  |
|                                                                |
|  UI BRIDGE                                                     |
|  +----------------------------------------------------------+  |
|  | Tauri commands + event emitter (status, stats display)   |  |
|  +----------------------------------------------------------+  |
|                                                                |
+================================================================+
```

---

## 5. Data Flow Diagrams

### 5.1 Key Press Event Flow (Happy Path)

```
  [Physical Keyboard]
         |
         | OS delivers to WH_KEYBOARD_LL hook
         v
  [Hook Callback Thread]
         |
         | Places event in lock-free ring buffer
         v
  [Input Capture Service]  ----suppresses event if routing to remote client
         |
         | Tokio async task reads from ring buffer
         v
  [Route Input Use Case]
         |
         | Queries LayoutEngine: "where is the cursor?"
         v
  [Layout Engine]
         |
         | Returns: active_client = Some(client_id)
         v
  [Input Router]
         |
         | Looks up client session in ClientRegistry
         v
  [Transmission Service]
         |
         | Translates: VK code -> HID Usage ID
         | Serializes: KEY_EVENT message with header
         | Encrypts: DTLS 1.3
         v
  [Network - UDP Socket]
         |
         | [LAN packet <1ms]
         v
  [Client - UDP Receiver]
         |
         | Decrypts: DTLS 1.3
         | Deserializes: KEY_EVENT message
         | Validates: sequence number, session token
         v
  [Handle Input Event Use Case]
         |
         | Translates: HID Usage ID -> Platform keycode
         v
  [Input Emulation Service]
         |
         | Platform API call (SendInput / XTestFakeKeyEvent / CGEventPost)
         v
  [Target Application receives keystroke]

  Total target latency: < 5ms P50, < 10ms P95
```

### 5.2 Edge Transition Flow

```
  Master cursor at position (1919, 540) -- approaching right edge of master screen (1920px wide)
         |
         | WH_MOUSE_LL fires with new position
         v
  [Input Capture Service]
         |
         | Posts MouseMoveEvent to queue
         v
  [Route Input Use Case]
         |
         | Passes position to LayoutEngine.check_transition(1919, 540)
         v
  [Layout Engine - Edge Detection]
         |
         | Checks: is cursor within EDGE_THRESHOLD (2px) of any screen boundary?
         | Finds: RIGHT edge of master screen is adjacent to LEFT edge of CLIENT_A
         | Returns: Transition { to: client_a, entry_x: 0, entry_y: 360 }
         |          (540/1080 * 720 = 360, proportional to CLIENT_A's 1440px height)
         v
  [Input Router - Transition Handler]
         |
         | 1. Updates active_client to CLIENT_A
         | 2. Sends MOUSE_MOVE(x=0, y=360) to CLIENT_A
         | 3. Calls SetCursorPos() to teleport master cursor to left edge
         |    (maintains illusion of continuous cursor movement)
         v
  [Subsequent mouse movements routed to CLIENT_A]
```

### 5.3 Device Discovery and Pairing Flow

```
  [Client - starts up]
         |
         | Broadcasts ANNOUNCE every 5 seconds on UDP:24802
         v
  [Master - DiscoveryResponder]
         |
         | Receives ANNOUNCE, sends ANNOUNCE_RESPONSE with control port
         | Adds client to "discovered but unpaired" list
         | UI notification: "New device found: dev-linux"
         v
  [User action in master UI: "Pair this device"]
         |
         | Master sends PAIRING_REQUEST (6-digit PIN displayed on screen)
         v
  [Client UI: prompts user to enter PIN]
         |
         | User enters PIN
         | Client sends PAIRING_RESPONSE with PIN
         v
  [Master - Pairing Handler]
         |
         | Validates PIN (hash comparison)
         | On success: stores client certificate, assigns layout position
         | On failure: increments attempt counter, lock out after 3 failures
         v
  [Client - Pairing Handler]
         |
         | Stores master certificate
         | Proceeds to SCREEN_INFO exchange
         v
  [Normal operation begins]
```

---

## 6. Layout Engine Design

### 6.1 Virtual Screen Space

The layout engine maintains a unified "virtual screen space" - a 2D coordinate system where all screens (master + clients) are positioned. The master screen is anchored at (0, 0).

```
  Virtual Screen Space (not drawn to scale):

  (-1920, -1080)
        +---------------------------+
        |    CLIENT_ABOVE           |  2560 x 1440
        |    (-1280, -1080)         |
        +---------------------------+
                    |
  (0,0)             |
  +------------------+             (2560, 0)
  |   MASTER         |             +---------------------------+
  |   1920 x 1080    | <-adjacent->|   CLIENT_RIGHT            |
  |                  |             |   2560 x 1440             |
  +------------------+             +---------------------------+
  (0, 1080)
```

### 6.2 Edge Detection Algorithm

```
EDGE_THRESHOLD = 2 pixels

function check_transition(cursor_x, cursor_y):
    master_rect = Rect(0, 0, master_width, master_height)

    for each (edge, adjacent_client) in layout.adjacencies:
        if cursor is within EDGE_THRESHOLD of edge:
            entry_point = map_proportional(cursor_pos, edge, adjacent_client.edge)
            return Transition(to=adjacent_client, entry=entry_point)

    return None (cursor stays on master or current client)

function map_proportional(pos, from_edge, to_edge):
    # Maps position along an edge proportionally to the target edge
    # E.g., 50% down the right edge of master -> 50% down the left edge of client
    t = (pos.y - from_edge.start) / from_edge.length
    return Point(
        x = to_edge.start_x + (t * to_edge.length) * direction_x,
        y = to_edge.start_y + (t * to_edge.length) * direction_y
    )
```

### 6.3 Layout Data Model

```rust
// Domain entities in kvm-core

pub struct VirtualLayout {
    pub master: ScreenRegion,
    pub clients: HashMap<ClientId, ClientScreen>,
    pub adjacencies: Vec<Adjacency>,
}

pub struct ScreenRegion {
    pub x: i32,      // Virtual space X offset
    pub y: i32,      // Virtual space Y offset
    pub width: u32,
    pub height: u32,
}

pub struct ClientScreen {
    pub client_id: ClientId,
    pub region: ScreenRegion,
    pub name: String,
}

pub struct Adjacency {
    pub from_screen: ScreenId,
    pub from_edge: Edge,   // Top, Bottom, Left, Right
    pub to_screen: ScreenId,
    pub to_edge: Edge,
}

pub enum Edge { Top, Bottom, Left, Right }
```

---

## 7. Security Architecture

### 7.1 Trust Model

```
  TRUST BOUNDARIES:

  [Physical Hardware]
       |
       | (trusted - operator controlled)
       v
  [Master OS + Application] --- encrypted ----> [Client OS + Application]
       |                                              |
       | (trusted after pairing)            (trusted after pairing)
       v                                              v
  [Master Key Storage]                      [Client Key Storage]
  (DPAPI encrypted)                         (Platform keychain)
```

### 7.2 Certificate Management

Each application instance generates a self-signed X.509 certificate on first run. Certificate exchange occurs during pairing:

1. Client presents its certificate during TLS handshake.
2. Master stores the certificate fingerprint (SHA-256) indexed by client_id.
3. On subsequent connections, master verifies the presented certificate matches the stored fingerprint.
4. Certificate rotation: Users can "re-pair" to rotate certificates.

### 7.3 Threat Model

| Threat | Mitigation |
|--------|-----------|
| Network eavesdropping | DTLS 1.3 encryption on all input events |
| Man-in-the-middle | Certificate pinning after initial TOFU pairing |
| Rogue client injection | Session token required; unpaired clients rejected |
| Replay attacks | DTLS built-in replay protection window |
| Key logging (malicious client) | Input events go TO clients, not FROM; master only sends input to paired clients |
| Local privilege escalation | Application runs without elevated privileges; Linux requires 'input' group (not root) |
| Brute-force PIN | 3 attempts then 60-second lockout per IP |

---

## 8. UI Design Specification

### 8.1 Master Application UI

**Main Window: Layout Editor**
```
+==================================================================+
|  KVM-Over-IP  [_][[][ X ]                                        |
+==================================================================+
|  [File] [Edit] [View] [Help]                                     |
+------------------------------------------------------------------+
|                                                                  |
|  Clients: [3 connected] [1 discovered]  Sharing: [ON/OFF]        |
|                                                                  |
|  +------------------------------------------------------------+  |
|  |  LAYOUT EDITOR  (drag to arrange)                          |  |
|  |                                                            |  |
|  |     +------------------+    +---------------------+        |  |
|  |     |  dev-linux       |    |  macbook            |        |  |
|  |     |  2560 x 1440     |    |  2560 x 1600        |        |  |
|  |     |  [connected]     |    |  [connected]        |        |  |
|  |     +------------------+    +---------------------+        |  |
|  |              +------------------+                          |  |
|  |              |  MASTER          |                          |  |
|  |              |  1920 x 1080     |                          |  |
|  |              |  [local]         |                          |  |
|  |              +------------------+                          |  |
|  |              +------------------+                          |  |
|  |              |  workstation-2   |                          |  |
|  |              |  1920 x 1200     |                          |  |
|  |              |  [connected]     |                          |  |
|  |              +------------------+                          |  |
|  |                                                            |  |
|  +------------------------------------------------------------+  |
|                                                                  |
|  CLIENT STATUS:                                                  |
|  +------------------------------------------------------------+  |
|  | Name         | IP           | Latency | Events/s | Status  |  |
|  |--------------|--------------|---------|----------|---------|  |
|  | dev-linux    | 192.168.1.10 | 2.1ms   | 0        | Active  |  |
|  | macbook      | 192.168.1.11 | 3.4ms   | 0        | Ready   |  |
|  | workstation-2| 192.168.1.12 | 1.8ms   | 145      | Routing |  |
|  +------------------------------------------------------------+  |
|                                                                  |
+==================================================================+
```

### 8.2 Client Application UI (System Tray Focus)

The client has a minimal UI - primarily a system tray icon with a status popup and a settings window accessible from the tray.

**Tray Icon States**:
- Grey circle: Disconnected / searching
- Green circle: Connected, idle
- Blue circle + pulse animation: Actively receiving input

**Status Popup** (click on tray icon):
```
+---------------------------+
|  KVM-Over-IP Client       |
|  Status: Connected        |
|  Master: 192.168.1.5      |
|  Latency: 2.1ms           |
|  Active: No               |
|  [Open Settings] [Quit]   |
+---------------------------+
```

---

## 9. Technical Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|-----------|
| Windows hook timeout causing hook removal | Medium | High | Hook callback must complete in <300ms; use lock-free queue to defer processing |
| Linux Wayland incompatibility | High | Medium | Document X11 requirement; provide Wayland roadmap for v2.0 |
| macOS Accessibility permission UX friction | High | Medium | Detailed in-app guidance with screenshots; auto-detect and prompt |
| DTLS library immaturity in Rust ecosystem | Medium | High | Evaluate `webrtc-dtls` crate vs OpenSSL FFI; prototype early in Phase 1 |
| Web client browser security restrictions | High | Low | Scope web client to DOM-only; document limitations prominently |
| Key code translation gaps (rare keys) | Low | Medium | Default to pass-through for unmapped codes; log warnings |
| Network clock skew affecting sequence numbers | Low | Low | Use per-connection sequence counters, not timestamps, for ordering |
| High CPU on master with many clients | Medium | Medium | Profile early; implement event batching (INPUT_BATCH message) |
