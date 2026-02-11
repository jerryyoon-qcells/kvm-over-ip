# Epics, Features, and User Stories
# KVM-Over-IP: Complete Agile Backlog

**Document Version**: 1.0
**Date**: 2026-02-10
**Status**: Ready for Sprint Planning

---

## Epic Summary

| Epic ID | Name | Story Points | Priority |
|---------|------|-------------|---------|
| EPIC-001 | Core Infrastructure & Protocol | 55 | Critical |
| EPIC-002 | Master Input Capture & Routing | 40 | Critical |
| EPIC-003 | Client Input Emulation (All Platforms) | 55 | Critical |
| EPIC-004 | Device Discovery & Pairing | 26 | Critical |
| EPIC-005 | Layout Engine & Configuration | 34 | Critical |
| EPIC-006 | Master UI (Layout Editor) | 34 | High |
| EPIC-007 | Client UI (Status & Settings) | 21 | High |
| EPIC-008 | Security & Encryption | 26 | Critical |
| EPIC-009 | Clipboard Sharing | 21 | Medium |
| EPIC-010 | Web Client | 34 | Medium |
| EPIC-011 | Packaging & Distribution | 26 | High |
| EPIC-012 | Testing & Quality Assurance | 34 | High |
| **TOTAL** | | **406** | |

---

# EPIC-001: Core Infrastructure & Protocol

**Epic ID**: EPIC-001
**Description**: Establish the foundational shared library (`kvm-core`) containing the network protocol, message codec, domain entities, and key translation tables. This epic is the prerequisite for all other epics.
**Business Value**: Without a correct, tested protocol implementation, no other component can be built. This is the foundation.
**Success Metrics**: All message types serializable/deserializable without data loss; key translation complete for 100% of standard keyboard keys.

---

## Feature: FEAT-001 - Protocol Message Codec

### US-001: Binary message encoding
**As a** developer implementing the master transmission service,
**I want** a binary serialization function for each protocol message type,
**So that** I can efficiently encode input events to transmit over the network.

**Acceptance Criteria**:
- [ ] Given any valid `KvmMessage`, `encode_message()` produces a byte slice with correct header and payload.
- [ ] Given the encoded byte slice, `decode_message()` returns the original message (round-trip equality).
- [ ] Given an empty or truncated byte slice, `decode_message()` returns `ProtocolError::InsufficientData`.
- [ ] Given a byte slice with an unknown message type byte, `decode_message()` returns `ProtocolError::UnknownMessageType`.
- [ ] Message header includes correct version (0x01), timestamp, and sequence number fields.

**Story Points**: 5
**Priority**: Critical
**Dependencies**: None

**Tasks**:
- [ ] Define all message structs in `kvm-core/src/protocol/messages.rs` - 4h
- [ ] Implement `encode_message` with bincode/serde serialization - 4h
- [ ] Implement `decode_message` with proper error handling - 4h
- [ ] Write unit tests for all message types (round-trip, error cases) - 6h
- [ ] Write property-based tests using proptest for fuzz-style coverage - 4h

---

### US-002: Message header sequence numbering
**As a** client receiving input events,
**I want** each message to carry a sequence number,
**So that** I can detect out-of-order or dropped packets.

**Acceptance Criteria**:
- [ ] Given a sequence of 1000 encoded messages, sequence numbers are monotonically increasing from 0.
- [ ] Given a `SequenceCounter`, `next()` is thread-safe (atomic increment).
- [ ] Sequence numbers wrap around correctly at u64::MAX without panic.

**Story Points**: 2
**Priority**: Critical
**Dependencies**: US-001

**Tasks**:
- [ ] Implement `SequenceCounter` with `AtomicU64` - 2h
- [ ] Integrate into `encode_message` function - 1h
- [ ] Unit tests: monotonicity, thread-safety, overflow behavior - 2h

---

## Feature: FEAT-002 - Key Code Translation Tables

### US-003: Windows VK to HID translation
**As a** master application,
**I want** to translate Windows Virtual Key codes to USB HID Usage IDs,
**So that** I can send platform-independent key codes to clients.

**Acceptance Criteria**:
- [ ] All 104 keys of a standard US QWERTY keyboard have correct VK -> HID mappings.
- [ ] Extended keys (numpad, function keys F1-F24, media keys) are mapped.
- [ ] `windows_vk_to_hid(unknown_code)` returns `HidKeyCode::Unknown` without panic.
- [ ] Bidirectional: `hid_to_windows_vk(hid)` returns the correct VK for all mapped keys.

**Story Points**: 5
**Priority**: Critical
**Dependencies**: US-001

**Tasks**:
- [ ] Research and compile complete VK-to-HID mapping table (reference USB HID spec) - 4h
- [ ] Implement `const` lookup array in `kvm-core/src/keymap/windows_vk.rs` - 4h
- [ ] Unit tests: all 104 standard keys, extended keys, bidirectionality, edge cases - 4h

---

### US-004: HID to Linux X11 KeySym translation
**As a** Linux client,
**I want** to translate received HID Usage IDs to X11 KeySyms,
**So that** I can inject keyboard events using the XTest extension.

**Acceptance Criteria**:
- [ ] All standard keyboard keys have HID -> X11 KeySym mappings.
- [ ] `hid_to_x11_keysym(HidKeyCode::Unknown)` returns `None` without panic.
- [ ] Extended/special keys (Print Screen, Pause, App key) are handled.

**Story Points**: 3
**Priority**: Critical
**Dependencies**: US-003

**Tasks**:
- [ ] Compile HID -> X11 KeySym mapping table - 3h
- [ ] Implement in `kvm-core/src/keymap/linux_x11.rs` - 2h
- [ ] Unit tests: standard keys, special keys, unknown codes - 2h

---

### US-005: HID to macOS CGKeyCode translation
**As a** macOS client,
**I want** to translate received HID Usage IDs to CGKeyCode values,
**So that** I can inject keyboard events using the CoreGraphics API.

**Acceptance Criteria**:
- [ ] All standard keyboard keys have HID -> CGKeyCode mappings.
- [ ] `hid_to_macos_cgkeycode(HidKeyCode::Unknown)` returns `None` without panic.

**Story Points**: 3
**Priority**: Critical
**Dependencies**: US-003

**Tasks**:
- [ ] Compile HID -> CGKeyCode mapping table (reference Carbon Events.h) - 3h
- [ ] Implement in `kvm-core/src/keymap/macos_cg.rs` - 2h
- [ ] Unit tests: standard keys, special keys, unknown codes - 2h

---

## Feature: FEAT-003 - Domain Entities

### US-006: VirtualLayout domain entity
**As a** developer implementing the routing use case,
**I want** a `VirtualLayout` domain entity with cursor resolution and edge detection,
**So that** routing logic is encapsulated and testable without infrastructure dependencies.

**Acceptance Criteria**:
- [ ] `VirtualLayout::resolve_cursor(x, y)` returns correct `CursorLocation` for all screen configurations.
- [ ] `check_edge_transition()` returns `Some(Transition)` when cursor is within 2px of an adjacent edge.
- [ ] `check_edge_transition()` returns `None` when cursor is not near any transition edge.
- [ ] `map_edge_position()` correctly maps proportional positions for screens of different heights.
- [ ] Adding two overlapping clients returns `LayoutError::Overlap`.

**Story Points**: 8
**Priority**: Critical
**Dependencies**: US-001

**Tasks**:
- [ ] Implement `VirtualLayout` struct and all methods - 8h
- [ ] Implement `ScreenRegion`, `Adjacency`, `EdgeTransition` structs - 2h
- [ ] Unit tests: single client, multi-client, edge cases, overlap detection - 8h
- [ ] Property-based tests for `map_edge_position` - 2h

---

# EPIC-002: Master Input Capture & Routing

**Epic ID**: EPIC-002
**Description**: Implement the Windows-specific input capture service using low-level hooks and the routing use case that directs captured events to the correct destination.
**Business Value**: This is the core master functionality. Without it, the application cannot intercept and forward any input.
**Success Metrics**: 100% of key presses and mouse events captured with < 1ms hook callback time; routing decisions made within 1ms of event receipt.

---

## Feature: FEAT-004 - Windows Input Capture

### US-007: Low-level keyboard hook
**As a** master operator,
**I want** the application to capture all keyboard input at the OS level,
**So that** I can route key presses to any connected client.

**Acceptance Criteria**:
- [ ] Given the master app running, all key presses (including Win key, Alt+Tab) are captured.
- [ ] Given routing to a client, captured keys are suppressed from the local OS (do not appear in local applications).
- [ ] Given routing disabled (hotkey), keys are not suppressed and function normally locally.
- [ ] Hook callback completes within 1ms (verified by timestamp comparison in tests).
- [ ] Hook remains installed for the lifetime of the application (no spurious removal).

**Story Points**: 8
**Priority**: Critical
**Dependencies**: FEAT-003

**Tasks**:
- [ ] Implement dedicated Win32 message loop thread in `infrastructure/input_capture/keyboard.rs` - 4h
- [ ] Implement `WH_KEYBOARD_LL` hook callback with ring buffer - 4h
- [ ] Implement hook suppression via `AtomicBool` flag - 2h
- [ ] Implement `InputCaptureService::start()` and `stop()` - 2h
- [ ] Integration test: verify events appear in receiver channel - 3h
- [ ] Performance test: measure hook callback duration - 2h

---

### US-008: Low-level mouse hook
**As a** master operator,
**I want** the application to capture all mouse movement and button events,
**So that** I can route mouse actions to connected clients and trigger edge transitions.

**Acceptance Criteria**:
- [ ] All mouse events (move, left/right/middle button, scroll, extra buttons) are captured.
- [ ] Mouse events include absolute screen coordinates.
- [ ] Mouse move events are captured at full resolution (no throttling by OS).
- [ ] Given routing to a client, mouse events are suppressed from the local OS.

**Story Points**: 5
**Priority**: Critical
**Dependencies**: US-007

**Tasks**:
- [ ] Implement `WH_MOUSE_LL` hook callback in `infrastructure/input_capture/mouse.rs` - 3h
- [ ] Add mouse events to the shared ring buffer - 1h
- [ ] Implement cursor teleportation (`SetCursorPos`) for edge transitions - 2h
- [ ] Integration test: verify mouse events appear in channel - 2h

---

## Feature: FEAT-005 - Input Routing Use Case

### US-009: Keyboard event routing
**As a** master operator,
**I want** keyboard events to automatically route to the correct client based on cursor position,
**So that** I can type on any client machine without switching applications.

**Acceptance Criteria**:
- [ ] Given cursor on client A's virtual area, all key presses route to client A only.
- [ ] Given cursor on master screen, key presses are not routed to any client.
- [ ] Given client A disconnects while active, routing falls back to master immediately.
- [ ] Routing decisions are made within 1ms of event receipt.

**Story Points**: 5
**Priority**: Critical
**Dependencies**: US-007, US-006

**Tasks**:
- [ ] Implement `RouteInputUseCase::handle_event()` for keyboard events - 4h
- [ ] Wire `InputCaptureService` output to `RouteInputUseCase` via async channel - 2h
- [ ] Unit tests with mock transmitter: verify correct client receives events - 4h
- [ ] Unit test: routing on disconnect falls back correctly - 2h

---

### US-010: Mouse event routing and edge transitions
**As a** master operator,
**I want** the mouse cursor to seamlessly transition between screens when reaching an edge,
**So that** I can move my mouse naturally across multiple computers as if they were one desktop.

**Acceptance Criteria**:
- [ ] Given adjacent screens in the layout, cursor reaching the shared edge transitions to the neighboring screen.
- [ ] Entry point on the neighboring screen is proportionally mapped (as per FR-003).
- [ ] Physical cursor teleports to the opposite edge of the master screen to allow continued movement.
- [ ] Transition occurs within 5ms (from edge detection to routing switch and cursor teleport).
- [ ] Rapid back-and-forth across an edge does not cause transition oscillation (debounce: 50ms cooldown).

**Story Points**: 8
**Priority**: Critical
**Dependencies**: US-008, US-006, US-009

**Tasks**:
- [ ] Integrate `VirtualLayout::check_edge_transition()` into mouse move handler - 3h
- [ ] Implement transition logic: update active target, send mouse move to new client, teleport cursor - 4h
- [ ] Implement 50ms debounce for transition to prevent oscillation - 2h
- [ ] Unit tests: transition at each edge direction, proportional mapping accuracy - 4h
- [ ] Manual QA: smooth cursor movement across 3-screen layout - 2h

---

### US-011: Sharing disable/enable hotkey
**As a** master operator,
**I want** a hotkey to instantly disable all input sharing,
**So that** I can quickly regain full local control in an emergency.

**Acceptance Criteria**:
- [ ] Double-tap Scroll Lock (default) disables sharing within 100ms.
- [ ] Visual indicator (tray icon change) confirms sharing is disabled.
- [ ] Pressing the hotkey again re-enables sharing.
- [ ] Hotkey is configurable via settings UI (US-031).
- [ ] When disabled, no events are routed to any client.

**Story Points**: 3
**Priority**: Critical
**Dependencies**: US-007, US-008

**Tasks**:
- [ ] Implement hotkey detection in keyboard hook callback (configurable VK code) - 3h
- [ ] Implement `SharingState` toggle with atomic flag - 1h
- [ ] Emit Tauri event on state change for UI update - 1h
- [ ] Unit tests: hotkey detection, state transitions - 2h

---

# EPIC-003: Client Input Emulation

**Epic ID**: EPIC-003
**Description**: Implement platform-specific input emulation on all supported client platforms: Windows, Linux (X11), macOS, and Web browser.
**Business Value**: The client is the delivery end of the system. Without working input emulation, the entire product has no value.
**Success Metrics**: Emulated input indistinguishable from physical input on all target platforms; emulation latency < 5ms after receipt.

---

## Feature: FEAT-006 - Windows Client Emulation

### US-012: Windows keyboard emulation
**As a** Windows client user,
**I want** keyboard events from the master to be injected into my OS,
**So that** I can use the master keyboard to type on my Windows machine.

**Acceptance Criteria**:
- [ ] Given a `KeyEventMessage`, `SendInput` is called with the correct VK/scan code and event type.
- [ ] Extended keys (arrows, F-keys, numpad) are emulated correctly with `KEYEVENTF_EXTENDEDKEY`.
- [ ] Modifier state (Ctrl, Shift, Alt, Meta) is correctly reflected in the injected events.
- [ ] Emulated input is accepted by standard Windows applications (Notepad, VS Code, browsers).

**Story Points**: 5
**Priority**: Critical
**Dependencies**: FEAT-002 (US-003)

**Tasks**:
- [ ] Implement `WindowsInputEmulator::emit_key_down/up()` using `SendInput` - 4h
- [ ] Handle HID -> Windows VK translation within emulator - 2h
- [ ] Handle extended key flags - 1h
- [ ] Unit tests using mock OS API wrapper - 4h
- [ ] Manual QA: type in Notepad, VS Code - 2h

---

### US-013: Windows mouse emulation
**As a** Windows client user,
**I want** mouse movements and clicks from the master to control my Windows machine's cursor,
**So that** the master can interact with applications on my machine.

**Acceptance Criteria**:
- [ ] `MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE` correctly positions cursor at received coordinates.
- [ ] Mouse button clicks (left, right, middle, X1, X2) are emulated correctly.
- [ ] Scroll events (`MOUSEEVENTF_WHEEL`) scroll the correct amount in the correct direction.
- [ ] Cursor positions are correctly normalized to Windows' 0-65535 virtual screen coordinate space.
- [ ] Multi-monitor scenarios: cursor position maps to correct monitor.

**Story Points**: 5
**Priority**: Critical
**Dependencies**: US-012

**Tasks**:
- [ ] Implement `WindowsInputEmulator::emit_mouse_move()` with coordinate normalization - 3h
- [ ] Implement `emit_mouse_button()` and `emit_mouse_scroll()` - 2h
- [ ] Implement virtual screen coordinate normalization helper - 2h
- [ ] Unit tests: coordinate normalization, multi-monitor offsets - 3h
- [ ] Manual QA: cursor positioning, click accuracy, scroll behavior - 2h

---

## Feature: FEAT-007 - Linux Client Emulation (X11/XTest)

### US-014: Linux keyboard emulation via XTest
**As a** Linux (X11) client user,
**I want** keyboard events from the master to be injected into my X session,
**So that** I can use the master keyboard to type on my Linux machine.

**Acceptance Criteria**:
- [ ] `XTestFakeKeyEvent` is called correctly for key_down and key_up events.
- [ ] HID Usage IDs are correctly translated to X11 KeySyms via lookup table.
- [ ] `XFlush` is called after each event to ensure immediate delivery.
- [ ] Application links against `libxtst` dynamically (not statically bundled).
- [ ] If X display connection is lost, `InputEmulationService` attempts reconnection.

**Story Points**: 5
**Priority**: Critical
**Dependencies**: FEAT-002 (US-004)

**Tasks**:
- [ ] Implement `LinuxXTestEmulator` in `infrastructure/input_emulation/linux.rs` - 4h
- [ ] Safe FFI bindings for xlib/xtst functions (or use `x11` crate) - 3h
- [ ] Implement HID -> X11 KeySym lookup and XTest call - 2h
- [ ] Unit tests: verify correct XTest function calls via mock - 3h
- [ ] Manual QA: type in terminal, gedit, VS Code on Ubuntu - 2h

---

### US-015: Linux mouse emulation via XTest
**As a** Linux (X11) client user,
**I want** mouse events from the master to control my X session cursor,
**So that** the master can interact with applications on my Linux machine.

**Acceptance Criteria**:
- [ ] `XTestFakeMotionEvent` correctly moves cursor to absolute coordinates.
- [ ] `XTestFakeButtonEvent` correctly presses/releases mouse buttons (1-5).
- [ ] Scroll emulation uses XTest button events (button 4/5 for vertical, 6/7 for horizontal).
- [ ] All events call `XFlush` for immediate delivery.

**Story Points**: 3
**Priority**: Critical
**Dependencies**: US-014

**Tasks**:
- [ ] Implement mouse move, button, and scroll in `LinuxXTestEmulator` - 3h
- [ ] Implement scroll wheel to XTest button mapping - 1h
- [ ] Unit tests with mock xlib display - 2h
- [ ] Manual QA: mouse precision, scrolling, right-click menus on Ubuntu - 2h

---

## Feature: FEAT-008 - macOS Client Emulation

### US-016: macOS keyboard emulation via CoreGraphics
**As a** macOS client user,
**I want** keyboard events from the master to be injected into my macOS session,
**So that** I can use the master keyboard to type on my Mac.

**Acceptance Criteria**:
- [ ] `CGEventCreateKeyboardEvent` and `CGEventPost` are called correctly.
- [ ] HID Usage IDs are correctly translated to CGKeyCode values.
- [ ] Accessibility permission is checked at startup; user is guided to grant it if missing.
- [ ] Modifier keys (Cmd, Option, Control, Shift) are emulated correctly.

**Story Points**: 8
**Priority**: Critical
**Dependencies**: FEAT-002 (US-005)

**Tasks**:
- [ ] Implement `MacosInputEmulator` in `infrastructure/input_emulation/macos.rs` - 4h
- [ ] Safe Rust bindings for CoreGraphics CGEvent APIs (via `core-graphics` crate) - 3h
- [ ] Implement Accessibility permission check and UI prompt flow - 3h
- [ ] Unit tests with mocked CoreGraphics (where feasible) - 2h
- [ ] Manual QA: type in TextEdit, Terminal, VS Code on macOS 13 - 2h

---

### US-017: macOS mouse emulation via CoreGraphics
**As a** macOS client user,
**I want** mouse events from the master to control my Mac's cursor,
**So that** the master can interact with applications on my Mac.

**Acceptance Criteria**:
- [ ] `CGEventCreateMouseEvent` correctly positions cursor and emulates button events.
- [ ] Scroll events use `CGEventCreateScrollWheelEvent` with correct delta values.
- [ ] Multi-monitor: cursor positions map correctly to macOS' coordinate system (origin at bottom-left on primary monitor).

**Story Points**: 5
**Priority**: Critical
**Dependencies**: US-016

**Tasks**:
- [ ] Implement mouse move, button, scroll in `MacosInputEmulator` - 3h
- [ ] Implement macOS coordinate system translation (flip Y axis) - 2h
- [ ] Unit tests - 2h
- [ ] Manual QA: cursor precision, scrolling, right-click on macOS 13 - 2h

---

## Feature: FEAT-009 - Concurrent Physical and Virtual Input

### US-018: Non-blocking local input on active client
**As a** local user of a client machine,
**I want** to use my local keyboard and mouse normally even when the master is routing input to my machine,
**So that** I am not locked out of my own computer.

**Acceptance Criteria**:
- [ ] Local keystrokes on a client are processed independently of master routing state.
- [ ] There is no blocking, queuing, or delay on local input caused by the client application.
- [ ] When both master and local user type simultaneously, both inputs reach applications (though behavior may be application-specific).
- [ ] Client application CPU impact during local-only use is below 1%.

**Story Points**: 3
**Priority**: Critical
**Dependencies**: US-012, US-014, US-016

**Tasks**:
- [ ] Verify `SendInput` / `XTestFakeKeyEvent` / `CGEventPost` do not block local input pipeline - 2h
- [ ] Test concurrent local + remote input on each platform - 3h
- [ ] Document any known OS limitations (e.g., event ordering is non-deterministic) - 1h

---

# EPIC-004: Device Discovery & Pairing

**Epic ID**: EPIC-004
**Description**: Implement the zero-configuration discovery mechanism (UDP broadcast/mDNS) and the secure PIN-based pairing process.
**Business Value**: Users should be able to add a new client in under 2 minutes with no manual IP configuration. Poor onboarding kills adoption.
**Success Metrics**: Discovery within 30 seconds of client startup; pairing completes in < 60 seconds with zero configuration errors.

---

## Feature: FEAT-010 - Device Discovery

### US-019: Client UDP broadcast announcement
**As a** client application,
**I want** to broadcast my presence on the local network every 5 seconds,
**So that** the master can discover me without manual IP entry.

**Acceptance Criteria**:
- [ ] Client broadcasts UDP ANNOUNCE message to 255.255.255.255:24802 every 5 seconds.
- [ ] ANNOUNCE includes: client_id, platform, control port, and human-readable name.
- [ ] Broadcasting stops when client successfully pairs and connects.
- [ ] Broadcasting resumes on disconnect.

**Story Points**: 3
**Priority**: Critical
**Dependencies**: FEAT-001

**Tasks**:
- [ ] Implement `DiscoveryBroadcaster` in `kvm-client/infrastructure/network/` - 3h
- [ ] Encode `AnnounceMessage` and send via UDP socket - 2h
- [ ] Start/stop logic tied to connection state - 1h
- [ ] Unit tests with mock socket - 2h

---

### US-020: Master discovery listener and UI notification
**As a** master operator,
**I want** to see newly discovered clients appear in the UI automatically,
**So that** I can pair with them without knowing their IP address.

**Acceptance Criteria**:
- [ ] Master listens on UDP:24802 for ANNOUNCE messages.
- [ ] On receipt, master sends ANNOUNCE_RESPONSE unicast back to client.
- [ ] Newly discovered (unpaired) client appears in master UI within 10 seconds.
- [ ] Already-paired clients are not shown as new discoveries; they connect automatically.
- [ ] Operator can manually add a client by IP:port as an alternative.

**Story Points**: 5
**Priority**: Critical
**Dependencies**: US-019

**Tasks**:
- [ ] Implement `DiscoveryResponder` in `kvm-master/infrastructure/network/` - 3h
- [ ] Distinguish already-paired vs new clients in discovery handler - 2h
- [ ] Emit Tauri event `client-discovered` to UI - 1h
- [ ] Implement manual IP entry in UI and backend - 2h
- [ ] Unit tests: paired client, new client, duplicate announcements - 3h

---

## Feature: FEAT-011 - Secure Pairing

### US-021: PIN-based pairing flow (master side)
**As a** master operator,
**I want** to pair with a discovered client by verifying a displayed PIN,
**So that** only authorized clients can receive my keyboard and mouse input.

**Acceptance Criteria**:
- [ ] Clicking "Pair" in master UI displays a 6-digit PIN in the master UI.
- [ ] PIN is valid for 60 seconds; expired PIN shows error and allows retry.
- [ ] After 3 wrong PIN entries from the same client IP, that IP is locked out for 60 seconds.
- [ ] On successful pairing, client certificate is persisted.
- [ ] UI updates to show client as "Paired" and prompts for layout placement.

**Story Points**: 5
**Priority**: Critical
**Dependencies**: US-020, EPIC-008 (US-036)

**Tasks**:
- [ ] Implement PIN generation (cryptographically random 6 digits) - 1h
- [ ] Implement pairing state machine in `ConnectionManager` - 4h
- [ ] Implement attempt counter and lockout - 2h
- [ ] Store client certificate fingerprint in `PairingStore` - 2h
- [ ] Emit Tauri events for pairing state to UI - 1h
- [ ] Unit tests: correct PIN, wrong PIN, timeout, lockout - 4h

---

### US-022: PIN-based pairing flow (client side)
**As a** client user,
**I want** to enter the PIN shown on the master to complete pairing,
**So that** my machine is authorized to receive input from the master.

**Acceptance Criteria**:
- [ ] Client UI shows a PIN entry prompt when a pairing request is received.
- [ ] Client sends `PairingResponse` with hashed PIN.
- [ ] On success, master certificate is pinned on client.
- [ ] On failure, client UI shows error and allows retry.

**Story Points**: 3
**Priority**: Critical
**Dependencies**: US-021

**Tasks**:
- [ ] Implement `PairingResponse` handling in client `SessionManager` - 3h
- [ ] Store master certificate fingerprint in client `PairingStore` - 2h
- [ ] Client UI: PIN entry dialog - 2h
- [ ] Unit tests: successful pairing, incorrect PIN - 2h

---

# EPIC-005: Layout Engine & Configuration

**Epic ID**: EPIC-005
**Description**: Implement the full layout engine (built on the domain entity from EPIC-001) including screen dimension reporting from clients, layout persistence, and configuration management.
**Business Value**: The layout is what makes the seamless multi-screen experience possible. Incorrect layout data means broken cursor transitions.
**Success Metrics**: Layout correctly maps cursor transitions for any valid screen arrangement; configuration survives application restarts.

---

## Feature: FEAT-012 - Screen Dimension Reporting

### US-023: Client screen enumeration and reporting
**As a** master layout engine,
**I want** clients to report their screen dimensions,
**So that** I can correctly map the virtual layout and proportional cursor transitions.

**Acceptance Criteria**:
- [ ] On connection, client sends `ScreenInfoMessage` with all connected monitor dimensions within 2 seconds.
- [ ] On monitor change (add/remove/resolution change), client resends within 5 seconds.
- [ ] Multi-monitor: all monitors with their relative offsets are reported.
- [ ] Master updates `VirtualLayout` and notifies UI on receipt.

**Story Points**: 5
**Priority**: Critical
**Dependencies**: FEAT-010 (US-020)

**Tasks**:
- [ ] Implement `WindowsScreenEnumerator` using `EnumDisplayMonitors` - 3h
- [ ] Implement `X11ScreenEnumerator` using XRandR - 3h
- [ ] Implement `MacosScreenEnumerator` using `NSScreen.screens` - 3h
- [ ] Implement `ScreenInfoService` with change detection loop - 3h
- [ ] Send `ScreenInfoMessage` on connect and on change - 2h
- [ ] Unit tests for each enumerator (mocked OS APIs) - 3h

---

### US-024: Layout persistence and reload
**As a** master operator,
**I want** my layout configuration to be saved automatically and restored on restart,
**So that** I do not need to reconfigure the layout every time I restart the application.

**Acceptance Criteria**:
- [ ] Layout is saved to `config.toml` within 1 second of any change.
- [ ] On startup, layout is loaded and applied before the UI renders.
- [ ] If `config.toml` is corrupted, the application starts with a default layout and logs a warning.
- [ ] Client positions are identified by `client_id` (UUID), not by hostname, to handle IP changes.

**Story Points**: 3
**Priority**: Critical
**Dependencies**: FEAT-012 (US-023)

**Tasks**:
- [ ] Implement `ConfigRepository` with TOML read/write using serde - 3h
- [ ] Implement default layout generation on first run - 1h
- [ ] Implement config validation and corruption recovery - 2h
- [ ] Unit tests: save, load, corrupt file recovery - 3h

---

### US-025: Configuration export and import
**As a** system administrator,
**I want** to export and import the master configuration,
**So that** I can back up my setup and restore it on a new machine.

**Acceptance Criteria**:
- [ ] "Export Config" in settings UI saves complete configuration (layout, clients, hotkeys) to a user-selected file.
- [ ] "Import Config" loads a previously exported file, validates it, and applies it.
- [ ] Imported config that references unknown clients shows a warning but still loads the layout geometry.
- [ ] Pairing credentials (certificate hashes) are NOT exported for security reasons.

**Story Points**: 3
**Priority**: Should Have
**Dependencies**: US-024

**Tasks**:
- [ ] Implement export function: serialize config to file at user-selected path - 2h
- [ ] Implement import function: deserialize, validate, apply - 2h
- [ ] Implement import validation with warning messages - 1h
- [ ] Exclude pairing credentials from export - 1h
- [ ] Unit tests: export/import round-trip, invalid file handling - 2h

---

# EPIC-006: Master UI (Layout Editor)

**Epic ID**: EPIC-006
**Description**: Build the React-based master application UI including the drag-and-drop layout editor, client status table, and settings panels.
**Business Value**: The UI is the operator's primary interface. Poor usability leads to configuration errors and abandonment.
**Success Metrics**: New users complete layout setup in < 5 minutes; layout editor works correctly for all valid arrangements.

---

## Feature: FEAT-013 - Layout Editor Component

### US-026: Drag-and-drop screen arrangement
**As a** master operator,
**I want** to drag client screen tiles in a 2D canvas to arrange them in the same spatial layout as my physical monitors,
**So that** cursor transitions match my actual desk layout.

**Acceptance Criteria**:
- [ ] Each connected client is represented as a resizable tile (proportional to its resolution).
- [ ] Tiles can be dragged freely on the canvas and snap to edges of adjacent tiles.
- [ ] Master screen is displayed as a fixed anchor tile.
- [ ] Layout changes are applied live (routing updates) and persisted automatically.
- [ ] Tile shows client name, resolution, and connection status.

**Story Points**: 13
**Priority**: High
**Dependencies**: US-023, US-024

**Tasks**:
- [ ] Create `LayoutEditor` React component with canvas - 6h
- [ ] Implement drag-and-drop with `@dnd-kit/core` or custom pointer events - 8h
- [ ] Implement edge-snapping algorithm for clean tile alignment - 4h
- [ ] Scale tiles proportionally to screen resolution - 2h
- [ ] Invoke `update_layout` Tauri command on drag end - 2h
- [ ] Listen to `layout-changed` and `client-status-changed` Tauri events - 2h
- [ ] Unit tests for edge-snapping calculations - 3h

---

### US-027: Client status table
**As a** master operator,
**I want** to see all connected clients and their status (latency, active) in a table,
**So that** I can monitor the health of all connections at a glance.

**Acceptance Criteria**:
- [ ] Status table shows: name, IP address, latency (ms), events/sec, and status for each client.
- [ ] Status updates every 100ms (latency is a rolling 1-second average).
- [ ] Active routing target is highlighted in the table.
- [ ] Disconnected clients are shown in a greyed-out state with a "Reconnecting" indicator.

**Story Points**: 5
**Priority**: High
**Dependencies**: US-026

**Tasks**:
- [ ] Create `ClientStatusTable` React component - 4h
- [ ] Subscribe to `stats-update` Tauri event (100ms interval) - 2h
- [ ] Style active/disconnected/reconnecting states - 2h
- [ ] Unit tests: component rendering with various states - 2h

---

## Feature: FEAT-014 - Settings UI

### US-028: Hotkey configuration
**As a** master operator,
**I want** to configure the hotkey used to enable/disable input sharing,
**So that** I can avoid conflicts with other software.

**Acceptance Criteria**:
- [ ] Settings UI has a hotkey recorder that captures the next key press when the user clicks "Record".
- [ ] The recorded hotkey is saved and applied immediately.
- [ ] Hotkey is validated to ensure it does not conflict with critical OS shortcuts.
- [ ] Default hotkey is clearly displayed and a "Reset to Default" button is available.

**Story Points**: 3
**Priority**: High
**Dependencies**: US-011

**Tasks**:
- [ ] Create `HotkeyRecorder` React component with key capture logic - 3h
- [ ] Invoke `set_hotkey` Tauri command on save - 1h
- [ ] Implement client-side validation for OS key conflicts - 2h
- [ ] Unit tests: hotkey recording, conflict detection - 2h

---

### US-029: Network settings panel
**As a** master operator,
**I want** to configure network ports and the bind address,
**So that** I can resolve port conflicts and work in complex network environments.

**Acceptance Criteria**:
- [ ] Settings panel shows current port values (control, input, discovery).
- [ ] Operator can change any port; changes require an application restart to take effect.
- [ ] Bind address can be changed (for machines with multiple network interfaces).
- [ ] Invalid port values (< 1024, > 65535, non-numeric) are rejected with error messages.

**Story Points**: 3
**Priority**: Medium
**Dependencies**: US-027

**Tasks**:
- [ ] Create `NetworkSettings` React component with form validation - 3h
- [ ] Persist settings via `update_settings` Tauri command - 1h
- [ ] Show restart-required notice when ports change - 1h
- [ ] Unit tests: validation edge cases - 2h

---

# EPIC-007: Client UI

**Epic ID**: EPIC-007
**Description**: Build the minimal client UI (system tray + settings window) for all native platforms.
**Business Value**: Clients need enough UI to show status and allow basic configuration without being intrusive.
**Success Metrics**: Client status visible at a glance from system tray; settings accessible within 2 clicks.

---

### US-030: System tray icon and status popup
**As a** client user,
**I want** to see the connection status in the system tray at all times,
**So that** I know if my machine is connected to a master without opening a window.

**Acceptance Criteria**:
- [ ] Tray icon changes color based on state: grey (disconnected), green (connected), blue (active routing).
- [ ] Clicking the tray icon opens a status popup with: master IP, latency, active status.
- [ ] Status popup has buttons: "Open Settings" and "Quit".
- [ ] Tray icon tooltip shows state description.

**Story Points**: 5
**Priority**: High
**Dependencies**: FEAT-011

**Tasks**:
- [ ] Implement Tauri system tray with 3 icon states - 3h
- [ ] Create `StatusPopup` React component - 3h
- [ ] Wire to `get_status` Tauri command and status events - 2h
- [ ] Cross-platform testing: Windows, Linux (GTK), macOS - 3h

---

### US-031: Client settings window
**As a** client user,
**I want** a settings window where I can configure the master IP (for manual connection) and other options,
**So that** I can use the client in environments where auto-discovery does not work.

**Acceptance Criteria**:
- [ ] Settings window: manual master IP entry, port, autostart toggle, log level.
- [ ] Manual IP is used if set; auto-discovery is used if empty.
- [ ] Settings are persisted to client config file.
- [ ] "Reconnect" button forces an immediate reconnection attempt.

**Story Points**: 5
**Priority**: High
**Dependencies**: US-030

**Tasks**:
- [ ] Create `ClientSettings` React component - 4h
- [ ] Implement form validation for IP address and port - 2h
- [ ] Invoke `update_settings` and `connect_to_master` Tauri commands - 2h
- [ ] Unit tests: form validation - 2h

---

# EPIC-008: Security & Encryption

**Epic ID**: EPIC-008
**Description**: Implement all cryptographic components: TLS/DTLS setup, certificate management, key storage, and the session authentication mechanism.
**Business Value**: Without encryption, user input (passwords, sensitive text) is exposed on the network. Security is non-negotiable.
**Success Metrics**: All communication encrypted; no plaintext input data visible in network captures; certificate pinning prevents MitM.

---

### US-032: TLS 1.3 control channel
**As a** security officer,
**I want** the master-client control channel to use TLS 1.3,
**So that** all configuration and session data is encrypted in transit.

**Acceptance Criteria**:
- [ ] `rustls` configured with TLS 1.3 only (no fallback to TLS 1.2).
- [ ] Self-signed certificate generated on first run using P-256 ECDSA.
- [ ] Certificate pinned after pairing; subsequent connections verify the pin.
- [ ] Connection rejected if certificate fingerprint does not match stored pin.

**Story Points**: 8
**Priority**: Critical
**Dependencies**: FEAT-011

**Tasks**:
- [ ] Implement certificate generation using `rcgen` crate - 3h
- [ ] Configure `rustls` server and client with self-signed certs - 4h
- [ ] Implement certificate fingerprint storage in `PairingStore` - 2h
- [ ] Implement pin verification on reconnect - 2h
- [ ] Integration test: MitM scenario rejected correctly - 3h

---

### US-033: DTLS 1.3 input channel
**As a** security officer,
**I want** the input event UDP channel to use DTLS 1.3,
**So that** keyboard and mouse events cannot be intercepted or injected.

**Acceptance Criteria**:
- [ ] DTLS 1.3 negotiated for all UDP input traffic.
- [ ] Session token from control channel is used to bind DTLS session to authenticated TCP session.
- [ ] DTLS replay protection window is enabled.
- [ ] Network capture (Wireshark) shows no plaintext input event content.

**Story Points**: 8
**Priority**: Critical
**Dependencies**: US-032

**Tasks**:
- [ ] Evaluate and select DTLS library: `openssl` via FFI or `webrtc-dtls` crate (prototype both) - 8h
- [ ] Implement DTLS server socket in master - 4h
- [ ] Implement DTLS client socket in native client - 4h
- [ ] Bind DTLS session to TCP session via pre-shared session token - 3h
- [ ] Integration test + Wireshark validation - 3h

---

### US-034: Secure key storage
**As a** security officer,
**I want** all cryptographic material (certificates, pairing hashes) to be stored securely,
**So that** credentials cannot be accessed by other applications or casual file inspection.

**Acceptance Criteria**:
- [ ] Windows: DPAPI-encrypted file for key material.
- [ ] Linux: OS keyring via `secret-service` D-Bus interface; fallback to user-permissions-only file.
- [ ] macOS: macOS Keychain for key material.
- [ ] Key material files are never readable by other OS users.

**Story Points**: 5
**Priority**: Critical
**Dependencies**: US-032

**Tasks**:
- [ ] Implement `KeyringRepository` trait with platform implementations - 6h
- [ ] Windows: use `winapi::um::dpapi` for DPAPI - 2h
- [ ] Linux: use `secret-service` crate - 2h
- [ ] macOS: use `security-framework` crate for Keychain - 2h
- [ ] Unit tests with mocked keyring - 3h

---

# EPIC-009: Clipboard Sharing

**Epic ID**: EPIC-009
**Description**: Implement bidirectional clipboard synchronization between master and active client.
**Business Value**: Copy-paste across machines dramatically increases productivity and is one of the most-requested features in KVM software.
**Success Metrics**: Clipboard sync works within 500ms; text, HTML, and image formats supported.

---

### US-035: Master-to-client clipboard push
**As a** master operator,
**I want** text I copy on my master machine to be available for paste on the active client,
**So that** I can copy-paste content from my master to any client seamlessly.

**Acceptance Criteria**:
- [ ] When operator copies text on the master, it is sent to the currently active client within 500ms.
- [ ] Clipboard is only synced to the currently active client (not all clients).
- [ ] Maximum clipboard size: 10MB. Larger content is ignored with a warning.
- [ ] Text and RTF formats supported; image (PNG) supported.

**Story Points**: 5
**Priority**: Should Have
**Dependencies**: US-032 (encryption must be in place before clipboard sync)

**Tasks**:
- [ ] Implement clipboard monitor on master (Windows: `AddClipboardFormatListener`) - 3h
- [ ] Serialize clipboard content to `ClipboardDataMessage` - 2h
- [ ] Send via control channel (TCP/TLS) to active client - 2h
- [ ] Implement clipboard write on client for each platform - 4h
- [ ] Unit tests: size limit, format handling - 3h

---

### US-036: Client-to-master clipboard push
**As a** master operator,
**I want** text copied on an active client to be available for paste on my master machine,
**So that** I can copy results from client applications and paste them into master applications.

**Acceptance Criteria**:
- [ ] When a user copies text on a client, it is sent to the master within 500ms.
- [ ] Only the active client triggers clipboard sync to master.
- [ ] Same size and format constraints as US-035.

**Story Points**: 5
**Priority**: Should Have
**Dependencies**: US-035

**Tasks**:
- [ ] Implement clipboard monitor on each platform client - 4h
- [ ] Send `ClipboardDataMessage` to master on copy event - 2h
- [ ] Implement clipboard write on master (Windows) - 2h
- [ ] Integration test: copy on Linux client, paste on Windows master - 3h

---

# EPIC-010: Web Client

**Epic ID**: EPIC-010
**Description**: Build the web browser client that connects to the master via WebSocket and provides DOM-level input injection.
**Business Value**: Web client enables use from machines where native installation is not possible (locked-down corporate desktops, Chromebooks).
**Success Metrics**: Web client connects and receives input within 30 seconds of opening; all limitations clearly documented.

---

### US-037: Web client WebSocket connection
**As a** web client user,
**I want** to connect to the master by entering a URL in my browser,
**So that** I can use the master keyboard to control a web application without installing software.

**Acceptance Criteria**:
- [ ] Web client served as static files from the master or web-bridge server.
- [ ] Connecting to `https://<master-ip>:24803` opens the web client.
- [ ] Web client completes the authentication handshake via WebSocket.
- [ ] Connection status displayed in the browser tab.

**Story Points**: 8
**Priority**: Medium
**Dependencies**: US-032 (TLS), FEAT-001

**Tasks**:
- [ ] Implement `kvm-web-bridge` WebSocket server in Rust - 6h
- [ ] Implement WSS -> DTLS/TLS protocol translation in bridge - 4h
- [ ] Build web client React app with TypeScript - 6h
- [ ] Implement WebSocket connection and auth handshake in browser - 4h
- [ ] Unit tests for web bridge protocol translation - 3h

---

### US-038: Web client DOM input injection
**As a** master operator,
**I want** keyboard events to be injected into the web client's browser,
**So that** I can type in web applications on the client machine.

**Acceptance Criteria**:
- [ ] Key events are dispatched as `KeyboardEvent` to the `document.body` in the browser.
- [ ] Mouse click events are dispatched as `MouseEvent` to the appropriate DOM element.
- [ ] Limitations are clearly documented: OS-level injection not possible in browser.
- [ ] Web client UI explains its limitations vs. native client.

**Story Points**: 5
**Priority**: Medium
**Dependencies**: US-037

**Tasks**:
- [ ] Implement keyboard event dispatch in web client TypeScript - 3h
- [ ] Implement mouse event dispatch - 2h
- [ ] Write clear in-app documentation of limitations - 1h
- [ ] Cross-browser testing: Chrome, Firefox, Edge - 3h

---

# EPIC-011: Packaging & Distribution

**Epic ID**: EPIC-011
**Description**: Create installer packages for all target platforms and set up the CI/CD build pipeline.
**Business Value**: Without proper packaging, users cannot install the software. Clean installers drive adoption.
**Success Metrics**: All installers install cleanly on target platforms with no manual steps beyond standard install flow.

---

### US-039: Windows MSI installers
**As a** Windows user,
**I want** standard MSI installers for master and client,
**So that** I can install them using the standard Windows installer flow.

**Acceptance Criteria**:
- [ ] Master MSI installs to `%PROGRAMFILES%\KVMOverIP\`.
- [ ] Client MSI installs to `%PROGRAMFILES%\KVMOverIPClient\`.
- [ ] Both installers create Start Menu shortcuts and optionally a desktop shortcut.
- [ ] Both installers create a system tray autostart registry entry (user-configurable in installer).
- [ ] Both installers include uninstall functionality (Add/Remove Programs).
- [ ] Installer size: master < 100MB, client < 60MB.

**Story Points**: 5
**Priority**: High
**Dependencies**: All core epics complete

**Tasks**:
- [ ] Configure `cargo-wix` for master and client - 4h
- [ ] Write WiX XML for shortcuts, registry entries, uninstall - 4h
- [ ] Implement code signing step in CI (test with self-signed cert) - 2h
- [ ] Test install/uninstall on Windows 10 and Windows 11 clean VMs - 3h

---

### US-040: Linux DEB, RPM, AppImage packages
**As a** Linux user,
**I want** DEB, RPM, and AppImage packages for the client,
**So that** I can install it on any major Linux distribution.

**Acceptance Criteria**:
- [ ] DEB package installs cleanly on Ubuntu 22.04 and Debian 12.
- [ ] RPM package installs cleanly on Fedora 38.
- [ ] AppImage runs on any Linux distribution with FUSE support (no install required).
- [ ] Post-install script adds user to `input` group and configures udev rules.
- [ ] Package includes desktop entry (.desktop file) and icon.

**Story Points**: 5
**Priority**: High
**Dependencies**: All core epics complete

**Tasks**:
- [ ] Configure `cargo-deb` and `cargo-rpm` - 3h
- [ ] Write post-install script for input group and udev rules - 2h
- [ ] Configure AppImage build with linuxdeployqt or AppImageTool - 3h
- [ ] Test on Ubuntu 22.04, Debian 12, Fedora 38 clean VMs - 3h

---

### US-041: macOS DMG/PKG installer
**As a** macOS user,
**I want** a standard DMG or PKG installer for the client,
**So that** I can install it using the standard macOS drag-to-Applications or installer flow.

**Acceptance Criteria**:
- [ ] DMG contains the application bundle; drag-to-Applications installs correctly.
- [ ] Application is signed with Apple Developer ID and notarized.
- [ ] First run guides user through Accessibility permission grant.
- [ ] App runs on macOS 12 Monterey and macOS 14 Sonoma (Apple Silicon + Intel).

**Story Points**: 5
**Priority**: High
**Dependencies**: All core epics complete

**Tasks**:
- [ ] Configure `cargo-bundle` for macOS app bundle - 2h
- [ ] Implement universal binary build (arm64 + x64 lipo) - 2h
- [ ] Create DMG using `create-dmg` tool - 1h
- [ ] Implement first-run Accessibility permission prompt - 2h
- [ ] Code signing and notarization in CI (requires Apple Developer account) - 3h
- [ ] Test on macOS 12 and 14 (Apple Silicon) - 2h

---

### US-042: CI/CD build pipeline
**As a** developer,
**I want** a CI/CD pipeline that builds and tests all targets on every commit,
**So that** build and test regressions are caught immediately.

**Acceptance Criteria**:
- [ ] GitHub Actions workflow builds all targets (Windows, Linux, macOS) on push to `main` and on all PRs.
- [ ] All unit tests run and must pass before merge.
- [ ] Integration tests run on each platform.
- [ ] Build artifacts (installers, AppImage) are archived for each successful main branch build.
- [ ] Code coverage report generated and badge displayed in README.

**Story Points**: 8
**Priority**: High
**Dependencies**: None (should be set up early in development)

**Tasks**:
- [ ] Create `.github/workflows/ci.yml` with matrix strategy - 4h
- [ ] Configure Rust toolchain caching for fast builds - 2h
- [ ] Configure Node.js/npm caching for UI builds - 1h
- [ ] Add artifact upload steps for installer packages - 2h
- [ ] Configure `cargo-tarpaulin` for code coverage reporting - 2h
- [ ] Add coverage badge to README - 1h

---

# EPIC-012: Testing & Quality Assurance

**Epic ID**: EPIC-012
**Description**: Comprehensive test suite including unit tests, integration tests, performance benchmarks, and cross-platform compatibility tests.
**Business Value**: A KVM tool that drops input events or causes cursor glitches is worse than useless. Testing is critical.
**Success Metrics**: >= 80% unit test coverage; all integration tests passing on all platforms in CI; P95 latency < 10ms validated.

---

### US-043: Integration test harness
**As a** developer,
**I want** an integration test framework that can simulate master-client interactions,
**So that** I can test the full protocol stack without physical hardware.

**Acceptance Criteria**:
- [ ] Test harness can spawn a mock master and mock client in the same process or as separate processes.
- [ ] Tests can inject raw input events and verify they are received and emulated on the mock client.
- [ ] Tests can simulate network disconnection and verify reconnection behavior.
- [ ] All integration tests run in < 5 minutes on CI hardware.

**Story Points**: 8
**Priority**: High
**Dependencies**: EPIC-001, EPIC-002, EPIC-003

**Tasks**:
- [ ] Design `TestHarness` struct with mock master and mock client - 4h
- [ ] Implement in-process network simulation (or use loopback) - 4h
- [ ] Implement network interruption simulation - 2h
- [ ] Write 20+ integration test cases covering all major flows - 8h

---

### US-044: Latency performance benchmarks
**As a** performance engineer,
**I want** automated latency benchmarks,
**So that** I can verify and track that we meet the < 10ms P95 requirement.

**Acceptance Criteria**:
- [ ] Benchmark measures P50, P95, P99 latency from event generation to emulation confirmation.
- [ ] Benchmark runs on loopback (baseline) and over actual LAN.
- [ ] Results reported in CI as a benchmark artifact.
- [ ] Benchmark alerts if P95 exceeds 10ms on loopback.

**Story Points**: 5
**Priority**: High
**Dependencies**: US-043

**Tasks**:
- [ ] Implement timestamped event tracer in master and client - 3h
- [ ] Implement benchmark runner (1000 events, measure timestamps) - 3h
- [ ] Add benchmark to CI with threshold check - 2h
- [ ] Run baseline benchmark and document results - 2h

---

### US-045: Cross-platform functional test matrix
**As a** QA engineer,
**I want** a documented test matrix with test cases and results for each supported platform,
**So that** we have evidence that the application works correctly on all targets.

**Acceptance Criteria**:
- [ ] Test matrix covers: install, startup, discovery, pairing, routing, edge transitions, disconnect/reconnect, clipboard.
- [ ] Matrix is tested on: Windows 10, Windows 11, Ubuntu 22.04, Fedora 38, macOS 13, macOS 14.
- [ ] All test cases pass before a version is marked as release-ready.
- [ ] Test results documented with screenshots for UI-related tests.

**Story Points**: 8
**Priority**: High
**Dependencies**: EPIC-011

**Tasks**:
- [ ] Write test matrix document with 40+ test cases - 4h
- [ ] Execute full matrix on all 6 OS targets (manual) - 16h
- [ ] Document failures and create bug tickets - 2h
- [ ] Re-test after bug fixes - 4h

---

## Story Point Summary by Epic

| Epic | Points | Sprints (10pts/sprint) |
|------|--------|------------------------|
| EPIC-001: Core Infrastructure | 26 | 2.6 |
| EPIC-002: Master Input Capture | 29 | 2.9 |
| EPIC-003: Client Emulation | 34 | 3.4 |
| EPIC-004: Discovery & Pairing | 16 | 1.6 |
| EPIC-005: Layout & Config | 16 | 1.6 |
| EPIC-006: Master UI | 24 | 2.4 |
| EPIC-007: Client UI | 18 | 1.8 |
| EPIC-008: Security | 26 | 2.6 |
| EPIC-009: Clipboard | 10 | 1.0 |
| EPIC-010: Web Client | 18 | 1.8 |
| EPIC-011: Packaging | 26 | 2.6 |
| EPIC-012: Testing & QA | 26 | 2.6 |
| **TOTAL** | **269** | **26.9 sprints** |

Note: Story points represent complexity and uncertainty, not hours. At a typical team velocity of 40 points per 2-week sprint (2 developers), this represents approximately 13.5 sprints (27 weeks).
