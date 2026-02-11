# Technical Specification
# KVM-Over-IP: Platform APIs, Technology Stack, and Implementation Guidelines

**Document Version**: 1.0
**Date**: 2026-02-10
**Status**: Approved for Development

---

## 1. Technology Stack

### 1.1 Master Application (Windows)

| Component | Technology | Version | Rationale |
|-----------|-----------|---------|-----------|
| Language | Rust | 1.75+ | Memory safety, zero-cost abstractions, excellent Windows FFI, ideal for low-latency systems. Eliminates entire classes of crashes at compile time. |
| UI Framework | Tauri v2 + React | Tauri 2.x, React 18 | Native desktop performance with web UI flexibility; single codebase for UI across master + potential future platforms; Tauri 2 supports Windows 10+. |
| Input Capture | Windows API (WinAPI crate) | N/A | WH_KEYBOARD_LL and WH_MOUSE_LL hooks provide kernel-level intercept before applications receive events. |
| Async Runtime | Tokio | 1.35+ | Industry-standard async runtime for Rust; excellent performance for network I/O. |
| TLS | rustls | 0.23+ | Pure-Rust TLS 1.3 implementation; no OpenSSL dependency; audited. |
| DTLS | openssl (DTLS) via FFI | 3.x | DTLS 1.3 support; mature, well-audited library. |
| Serialization | bincode / serde | 2.x / 1.x | Binary serialization for network messages; zero-copy deserialization possible. |
| Configuration | toml | 0.8+ | Human-readable configuration format; serde support. |
| Logging | tracing + tracing-subscriber | 0.1.x | Structured, async-aware logging; configurable output. |
| Testing | built-in (cargo test) + mockall | N/A | Unit and integration testing; mockall for dependency injection. |

### 1.2 Client Application (Native: Windows, Linux, macOS)

| Component | Technology | Version | Rationale |
|-----------|-----------|---------|-----------|
| Language | Rust | 1.75+ | Same codebase shared with master for network protocol and core logic; platform-specific input emulation via conditional compilation. |
| UI Framework | Tauri v2 + React | Same as master | Consistent UI implementation across all platforms; system tray support. |
| Input Emulation | Platform-specific (see Section 2) | N/A | Must use OS APIs for realistic emulation. |
| TLS / DTLS | rustls + openssl | Same as master | Same libraries ensure protocol compatibility. |
| Async Runtime | Tokio | Same as master | Consistent async model. |

### 1.3 Web Client

| Component | Technology | Version | Rationale |
|-----------|-----------|---------|-----------|
| Language | TypeScript | 5.x | Type safety for complex protocol state machine. |
| Framework | React | 18.x | Consistency with master/client UI; large ecosystem. |
| Build Tool | Vite | 5.x | Fast builds; excellent TypeScript support. |
| WebSocket | Native browser WebSocket | N/A | TLS-wrapped WebSocket (WSS) for web client transport. |
| Input Injection | DOM EventTarget APIs | N/A | Browser-only input injection; limited to DOM events. |
| State Management | Zustand | 4.x | Lightweight, TypeScript-first state management. |

**Web Client Architecture Note**: The web client connects to a WebSocket proxy server (lightweight Rust binary or included in master) that bridges WSS to the native DTLS input channel. This is necessary because browsers cannot use raw UDP sockets.

### 1.4 Shared Library (Core Protocol)

A shared `kvm-core` Rust library crate contains:
- Protocol message serialization/deserialization
- Network connection management
- Encryption wrappers
- Key code translation tables (Windows VK -> HID -> Platform)

This crate is used by both master and native client applications.

---

## 2. Platform-Specific Input APIs

### 2.1 Input Capture on Windows (Master)

#### Keyboard Capture
```
API: SetWindowsHookEx(WH_KEYBOARD_LL, ...)
Hook type: Low-level global keyboard hook
Thread requirement: Dedicated Win32 message loop thread
Key data: KBDLLHOOKSTRUCT (vkCode, scanCode, flags, time)
```

**Implementation Notes**:
- The hook must run in a thread with a running Win32 message loop (`GetMessage` / `DispatchMessage` loop).
- The hook callback must return quickly (< 1ms) to avoid Windows timeout and hook removal. Event data is placed in a lock-free queue; processing happens on the Tokio async runtime.
- To suppress an event from reaching the local system, return a non-zero value from the hook callback (do not call `CallNextHookEx`).
- `WH_KEYBOARD_LL` hooks can intercept Win key, Alt+Tab, etc. when the process has sufficient privilege, but not Ctrl+Alt+Delete (hardware-level).

#### Mouse Capture
```
API: SetWindowsHookEx(WH_MOUSE_LL, ...)
Hook type: Low-level global mouse hook
Key data: MSLLHOOKSTRUCT (pt, mouseData, flags, time)
```

**Cursor Control for Edge Transitions**:
- `SetCursorPos(x, y)`: Teleports cursor to opposite edge on transition.
- `ClipCursor(LPRECT)`: Optional; can restrict cursor to master screen area when routing to client (prevents accidental local interaction).

### 2.2 Input Emulation on Windows (Client)

```
API: SendInput(cInputs, LPINPUT, cbSize)
Structure: INPUT with type INPUT_KEYBOARD or INPUT_MOUSE
Keyboard: KEYBDINPUT (wVk, wScan, dwFlags)
Mouse: MOUSEINPUT (dx, dy, mouseData, dwFlags, time)
```

**Important Flags**:
- `KEYEVENTF_EXTENDEDKEY`: For extended keys (numpad, arrows, etc.)
- `KEYEVENTF_SCANCODE`: Send scan code instead of VK code for more reliable injection.
- `MOUSEEVENTF_ABSOLUTE + MOUSEEVENTF_MOVE`: Absolute position mouse move.
- For absolute mouse: coordinates must be in the range [0, 65535] mapped to the virtual screen.

### 2.3 Input Emulation on Linux (Client)

#### X11 Input Emulation (Primary - v1.0)
```
Library: X11 XTest extension (libxtst)
APIs:
  XTestFakeKeyEvent(display, keycode, is_press, delay)
  XTestFakeMotionEvent(display, screen, x, y, delay)
  XTestFakeButtonEvent(display, button, is_press, delay)
```

**Key Code Translation**: HID Usage ID -> X11 Keycode requires an XKB mapping lookup. The `xkbcommon` library provides HID to X11 translation tables.

#### uinput Emulation (Alternative - Privilege Required)
```
Device: /dev/uinput
APIs: Linux kernel uinput interface
Setup: ioctl(fd, UI_SET_KEYBIT, key_code) for each key
Use: write(fd, input_event) to inject events
```

The uinput approach works at kernel level and is Wayland-compatible (v2.0 target), but requires the user to be in the `input` group.

#### Permission Setup (Linux Client Installer)
```bash
sudo usermod -a -G input $USER
sudo tee /etc/udev/rules.d/99-kvm-overip.rules << EOF
KERNEL=="uinput", GROUP="input", MODE="0660"
EOF
sudo udevadm control --reload-rules
```

### 2.4 Input Emulation on macOS (Client)

```
Framework: CoreGraphics (CGEvent API)
APIs:
  CGEventCreateKeyboardEvent(source, virtualKey, keyDown)
  CGEventPost(kCGHIDEventTap, event)
  CGEventCreateMouseEvent(source, type, location, button)
```

**HID to macOS Key Code**: Custom translation table mapping USB HID Usage IDs to `CGKeyCode` values (macOS virtual key codes, documented in Carbon Events.h).

**Permission Requirement**: `CGEventPost` to `kCGHIDEventTap` requires Accessibility permission. The client must check `AXIsProcessTrusted()` at startup and prompt the user if not granted.

**macOS Permission Check Code Pattern**:
```
if (!AXIsProcessTrustedWithOptions({kAXTrustedCheckOptionPrompt: true})) {
    // Display guidance to grant permission in System Preferences
    // -> Privacy & Security -> Accessibility
}
```

### 2.5 Web Client Input Emulation

The web client operates within strict browser security sandboxing. Input emulation is limited to:

```javascript
// Keyboard injection (within focused DOM element)
element.dispatchEvent(new KeyboardEvent('keydown', { key, code, bubbles: true }))

// Mouse events
document.dispatchEvent(new MouseEvent('mousemove', { clientX, clientY, bubbles: true }))
document.dispatchEvent(new MouseEvent('click', { ... }))
```

**Limitations**:
- Cannot inject events that affect OS-level focus or other browser windows.
- Cannot simulate real OS-level input (clipboard via keyboard shortcut will not work without Clipboard API).
- Primarily useful for web applications running in the browser tab, not as a full OS control mechanism.
- The web client is useful for monitoring and basic UI control, not as a replacement for native clients.

---

## 3. Architecture Patterns

### 3.1 Clean Architecture Layers

All applications follow Clean Architecture (Robert C. Martin):

```
+----------------------------------------------------------+
|                    Presentation Layer                    |
|           (Tauri commands, React components)             |
+----------------------------------------------------------+
|                   Application Layer                      |
|     (Use cases: RouteInput, ConnectClient, etc.)         |
+----------------------------------------------------------+
|                     Domain Layer                         |
|    (Entities: InputEvent, Layout, Client, Connection)    |
+----------------------------------------------------------+
|                  Infrastructure Layer                    |
|   (OS input capture/emulation, Network I/O, Storage)     |
+----------------------------------------------------------+
```

**Dependency Rule**: Dependencies point inward only. The domain layer has no knowledge of infrastructure.

### 3.2 Master Application Internal Architecture

```
+-------------------+     +------------------+     +-------------------+
|  Input Capture    |     |   Layout Engine  |     |  UI (Tauri/React) |
|  Service          |     |                  |     |                   |
| - WH_KEYBOARD_LL  | --> | - Screen mapping |<--> | - Layout editor   |
| - WH_MOUSE_LL     |     | - Edge detection |     | - Client list     |
| - Event queue     |     | - Route resolver |     | - Status/stats    |
+-------------------+     +------------------+     +-------------------+
         |                        |
         v                        v
+-------------------+     +------------------+
|  Input Router     |     |  Connection Mgr  |
|                   |     |                  |
| - Active target   |<--> | - Client registry|
| - Local bypass    |     | - TLS sessions   |
| - Transition logic|     | - DTLS sessions  |
+-------------------+     +------------------+
         |                        |
         v                        v
+---------------------------------------------------+
|              Transmission Service                  |
|   - Input channel sender (UDP/DTLS)               |
|   - Clipboard sync                                 |
|   - Sequence numbering                             |
+---------------------------------------------------+
```

### 3.3 Client Application Internal Architecture

```
+-------------------+     +------------------+     +-------------------+
|  Connection       |     | Input Emulation  |     |  UI (Tauri/React) |
|  Manager          |     | Service          |     |                   |
| - TLS control     | --> | - Platform API   |     | - Status display  |
| - DTLS input      |     | - Event queue    |     | - Settings        |
| - Reconnect logic |     | - Dedup filter   |     | - Tray icon       |
+-------------------+     +------------------+     +-------------------+
         |
         v
+-------------------+
|  Screen Reporter  |
|                   |
| - Monitor enum.   |
| - Change detect   |
| - Report sender   |
+-------------------+
```

---

## 4. Key Code Translation Specification

The canonical cross-platform key representation is USB HID Usage IDs (page 0x07, Keyboard/Keypad page). All platform-specific codes must be translated to/from this canonical form.

### 4.1 Translation Tables Required

| Translation | Direction | Used In |
|-------------|-----------|---------|
| Windows VK -> HID | Input | Master (capture) |
| HID -> Windows VK | Output | Windows Client (emulation) |
| HID -> X11 Keycode | Output | Linux Client (emulation) |
| HID -> CGKeyCode | Output | macOS Client (emulation) |
| HID -> DOM KeyboardEvent.code | Output | Web Client (emulation) |

### 4.2 Translation Table Format

Translation tables are stored as compile-time constant arrays in the `kvm-core` crate:

```rust
// Example: First 5 entries of Windows VK -> HID table
const VK_TO_HID: [(u8, u8); 256] = [
    (0x00, 0x00), // Reserved
    (0x01, 0x00), // VK_LBUTTON -> no HID keyboard equivalent
    // ...
    (0x41, 0x04), // VK_A -> HID Usage 0x04 (Keyboard a)
    // ...
];
```

---

## 5. Configuration File Specification

**Location**:
- Windows: `%APPDATA%\KVMOverIP\config.toml`
- Linux: `~/.config/kvmoverip/config.toml`
- macOS: `~/Library/Application Support/KVMOverIP/config.toml`

**Master Configuration Schema** (TOML):
```toml
[master]
version = "1.0"
disable_hotkey = "ScrollLock+ScrollLock"  # Double-tap Scroll Lock
autostart = true
log_level = "info"

[network]
control_port = 24800
input_port = 24801
discovery_port = 24802
bind_address = "0.0.0.0"

[layout]
master_screen_width = 1920
master_screen_height = 1080
# Client positions are relative to master top-left (0,0)
# Positive X = right, Positive Y = below

[[layout.clients]]
client_id = "550e8400-e29b-41d4-a716-446655440000"
name = "dev-linux"
x_offset = 1920      # Placed to the right of master
y_offset = 0
width = 2560
height = 1440

[[clients]]
client_id = "550e8400-e29b-41d4-a716-446655440001"
name = "macbook"
host = "192.168.1.105"  # Optional manual IP
pairing_hash = "sha256:abc123..."  # Derived from pairing PIN exchange
```

**Client Configuration Schema** (TOML):
```toml
[client]
version = "1.0"
autostart = true
log_level = "info"

[network]
control_port = 24800
input_port = 24801
master_host = ""   # Empty = auto-discover
```

---

## 6. Testing Strategy

### 6.1 Unit Testing

All pure business logic (layout engine, routing decisions, message serialization) must have unit tests achieving >= 80% code coverage.

**Key Unit Test Areas**:
- `layout_engine`: Edge detection math, coordinate mapping, multi-monitor offset handling.
- `protocol_codec`: Round-trip encode/decode for every message type with edge cases (zero-length fields, max values, boundary values).
- `key_translation`: Verify bidirectional translation for all keys in the translation table; test that unknown keys return a defined "unknown" sentinel.
- `input_router`: Route selection logic for all layout configurations (1 client, max clients, client with gap).

### 6.2 Integration Testing

- **Master-Client Connection**: Automated test spawning a mock client, completing the full handshake, and verifying SCREEN_INFO exchange.
- **Input Transmission**: Master sends 1000 key events; mock client confirms all are received with correct sequence numbers.
- **Edge Transition**: Simulated cursor approaching and crossing screen edges; verify routing target changes correctly.
- **Reconnection**: Drop the mock client network connection; verify master reconnects within 15 seconds.

### 6.3 Cross-Platform Testing Matrix

| Test Suite | Windows | Linux (Ubuntu 22.04) | macOS 13 |
|------------|---------|---------------------|----------|
| Unit tests | CI (GitHub Actions) | CI | CI |
| Integration tests | CI | CI | CI |
| Input emulation (manual) | Manual QA | Manual QA | Manual QA |
| Permission flow (manual) | Manual QA | Manual QA | Manual QA |

### 6.4 Performance Testing

- **Latency benchmark**: Measure P50/P95/P99 latency using hardware timer (loopback, LAN, WiFi).
- **Throughput test**: 1000 events/second for 60 seconds; measure CPU and memory.
- **Scalability test**: 16 simultaneous mock clients; measure master CPU and per-client latency.

---

## 7. Build and Packaging

### 7.1 Build System

- **Rust workspace**: Single `Cargo.toml` workspace containing `kvm-master`, `kvm-client`, `kvm-core`, `kvm-web-bridge` crates.
- **Node/npm**: Separate workspace for React UI (`packages/ui-master`, `packages/ui-client`).
- **CI**: GitHub Actions with matrix builds for each target platform.

### 7.2 Build Targets and Artifacts

| Target | Artifact | Tool |
|--------|----------|------|
| Windows x64 (master) | kvm-master.msi | WiX Toolset via cargo-wix |
| Windows x64 (client) | kvm-client.msi | WiX Toolset |
| Linux x64 (client) | kvm-client.deb, .rpm, .AppImage | cargo-deb, cargo-rpm, AppImageTool |
| macOS arm64 + x64 (client) | kvm-client.dmg | cargo-bundle |
| Web (client) | dist/ (static assets) | Vite build |

### 7.3 Code Signing

- Windows: Authenticode code signing (certificate required for distribution).
- macOS: Apple Developer ID signing and notarization (required for Gatekeeper).
- Linux: GPG signing of .deb/.rpm packages.

---

## 8. Logging and Observability

### 8.1 Log Levels and Content

| Level | Content | Default |
|-------|---------|---------|
| ERROR | Crashes, unrecoverable errors | Always on |
| WARN | Recoverable errors, reconnections | Always on |
| INFO | Connection events, pairing, layout changes | Default |
| DEBUG | Message sent/received (without key content), timing | Off |
| TRACE | Full message content (SENSITIVE - development only) | Off |

### 8.2 Key Content Masking

All log statements involving key codes must use a masking wrapper:
```rust
tracing::debug!("Key event: type={}, code=<masked>", event.event_type);
```
Key codes are never logged at INFO level or above.

### 8.3 Log File Location

- Windows: `%APPDATA%\KVMOverIP\logs\kvm-<date>.log`
- Linux: `~/.local/share/kvmoverip/logs/kvm-<date>.log`
- macOS: `~/Library/Logs/KVMOverIP/kvm-<date>.log`
- Rotation: Daily, retain last 7 days, max 100MB per file.
