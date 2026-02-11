# Network Protocol Specification
# KVM-Over-IP: Master-Client Communication Protocol v1.0

**Document Version**: 1.0
**Date**: 2026-02-10
**Status**: Approved for Development

---

## 1. Overview

The KVM-Over-IP protocol defines two communication channels between master and client:

1. **Control Channel** (TCP, TLS 1.3): Reliable, ordered channel for pairing, configuration exchange, screen dimension reporting, and connection lifecycle management.
2. **Input Channel** (UDP, DTLS 1.3): Low-latency, best-effort channel for real-time input event transmission. Sequence numbers enable ordering and loss detection without retransmission of stale events.

**Default Ports**:
- Control Channel: TCP 24800
- Input Channel: UDP 24801
- Discovery: UDP 24802 (multicast/broadcast)

---

## 2. Transport Layer

### 2.1 Control Channel (TCP + TLS 1.3)

The control channel uses TLS 1.3 (RFC 8446) with the following requirements:

- **Minimum TLS version**: 1.3
- **Cipher suites**: TLS_AES_256_GCM_SHA384, TLS_CHACHA20_POLY1305_SHA256
- **Certificate**: Self-signed X.509 v3, 4096-bit RSA or P-256 ECDSA, 1-year validity
- **Verification**: Certificate pinning after initial pairing (TOFU - Trust On First Use)
- **Keepalive**: TCP keepalive enabled; application-level ping every 5 seconds, timeout after 15 seconds

### 2.2 Input Channel (UDP + DTLS 1.3)

The input channel uses DTLS 1.3 (RFC 9147) with the following requirements:

- **Minimum DTLS version**: 1.3
- **Same cipher suites as control channel**
- **MTU**: Target 1400 bytes per datagram to avoid fragmentation
- **Retransmission**: No application-level retransmission for input events (stale input is worse than lost input)
- **Replay protection**: DTLS built-in replay window of 64 packets

### 2.3 Discovery Protocol (UDP Broadcast/Multicast)

Discovery uses UDP datagrams on port 24802:
- **Multicast group**: 224.0.0.251 (shared with mDNS)
- **Broadcast fallback**: 255.255.255.255 if multicast unavailable
- **Announcement interval**: Client broadcasts every 5 seconds while not paired
- **Response**: Master unicasts a response to the client's IP/port

---

## 3. Message Format

All messages use a binary framing format with a common header.

### 3.1 Common Message Header

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|    Version    |  Message Type |           Reserved            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         Payload Length                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       Sequence Number                         |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Timestamp (us)                         |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

- **Version** (1 byte): Protocol version. Current: 0x01
- **Message Type** (1 byte): See Section 3.2
- **Reserved** (2 bytes): Must be 0x0000
- **Payload Length** (4 bytes, big-endian): Length of payload in bytes (not including header)
- **Sequence Number** (8 bytes, big-endian): Monotonically increasing per-channel counter
- **Timestamp** (8 bytes, big-endian): Microseconds since Unix epoch at time of generation

**Total header size**: 24 bytes

### 3.2 Message Type Codes

#### Control Channel Messages (0x00 - 0x3F)

| Code | Name | Direction | Description |
|------|------|-----------|-------------|
| 0x01 | HELLO | Client -> Master | Initial connection handshake |
| 0x02 | HELLO_ACK | Master -> Client | Handshake acknowledgment with session token |
| 0x03 | PAIRING_REQUEST | Master -> Client | Initiate pairing with PIN |
| 0x04 | PAIRING_RESPONSE | Client -> Master | Accept/reject pairing |
| 0x05 | SCREEN_INFO | Client -> Master | Report screen dimensions |
| 0x06 | SCREEN_INFO_ACK | Master -> Client | Acknowledge screen info |
| 0x07 | PING | Both | Keepalive ping |
| 0x08 | PONG | Both | Keepalive response |
| 0x09 | DISCONNECT | Both | Graceful disconnect notification |
| 0x0A | ERROR | Both | Error notification |
| 0x0B | CLIPBOARD_DATA | Both | Clipboard content transfer |
| 0x0C | CONFIG_UPDATE | Master -> Client | Push configuration changes |

#### Input Channel Messages (0x40 - 0x7F)

| Code | Name | Direction | Description |
|------|------|-----------|-------------|
| 0x40 | KEY_EVENT | Master -> Client | Keyboard key press/release |
| 0x41 | MOUSE_MOVE | Master -> Client | Mouse absolute position |
| 0x42 | MOUSE_BUTTON | Master -> Client | Mouse button press/release |
| 0x43 | MOUSE_SCROLL | Master -> Client | Mouse wheel scroll event |
| 0x44 | INPUT_BATCH | Master -> Client | Batched input events (optimization) |

#### Discovery Messages (0x80 - 0x8F)

| Code | Name | Direction | Description |
|------|------|-----------|-------------|
| 0x80 | ANNOUNCE | Client -> Broadcast | Client announcing presence |
| 0x81 | ANNOUNCE_RESPONSE | Master -> Client | Master responding to announce |

---

## 4. Message Payload Specifications

### 4.1 HELLO (0x01)

Sent by client to master when initiating connection.

```
+------------------+----+-------------------------------------------+
| Field            | Bytes | Description                            |
+------------------+----+-------------------------------------------+
| client_id        | 16  | UUID v4 identifying this client instance |
| protocol_version | 1   | Protocol version supported by client     |
| platform_id      | 1   | 0x01=Windows, 0x02=Linux, 0x03=macOS,   |
|                  |     | 0x04=Web                                 |
| client_name_len  | 2   | Length of client_name string (UTF-8)     |
| client_name      | var | Human-readable client hostname           |
| capabilities     | 4   | Bitmask of supported features (see 4.1a) |
+------------------+----+-------------------------------------------+
```

**Capabilities Bitmask (4.1a)**:
- Bit 0: Keyboard emulation supported
- Bit 1: Mouse emulation supported
- Bit 2: Clipboard sharing supported
- Bit 3: Multi-monitor reporting supported
- Bits 4-31: Reserved

### 4.2 HELLO_ACK (0x02)

```
+------------------+-------+------------------------------------------+
| Field            | Bytes | Description                              |
+------------------+-------+------------------------------------------+
| session_token    | 32    | Random session token for input channel   |
| server_version   | 1     | Protocol version used by master          |
| accepted         | 1     | 0x01=connection accepted, 0x00=rejected  |
| reject_reason    | 1     | Reason code if rejected (0x00 if accepted)|
+------------------+-------+------------------------------------------+
```

### 4.3 SCREEN_INFO (0x05)

```
+------------------+-------+------------------------------------------+
| Field            | Bytes | Description                              |
+------------------+-------+------------------------------------------+
| monitor_count    | 1     | Number of monitors (1-16)               |
| monitors[]       | var   | Array of MonitorInfo structures          |
+------------------+-------+------------------------------------------+

MonitorInfo structure (per monitor):
+------------------+-------+------------------------------------------+
| monitor_id       | 1     | Monitor index (0-based)                  |
| x_offset         | 4     | X position relative to primary (pixels) |
| y_offset         | 4     | Y position relative to primary (pixels) |
| width            | 4     | Width in pixels                          |
| height           | 4     | Height in pixels                         |
| scale_factor     | 2     | DPI scale * 100 (e.g., 150 = 150%)      |
| is_primary       | 1     | 0x01 if primary monitor                  |
+------------------+-------+------------------------------------------+
```

### 4.4 KEY_EVENT (0x40)

```
+------------------+-------+------------------------------------------+
| Field            | Bytes | Description                              |
+------------------+-------+------------------------------------------+
| key_code         | 2     | USB HID keycode (platform-independent)   |
| scan_code        | 2     | Platform scan code (informational)       |
| event_type       | 1     | 0x01=key_down, 0x02=key_up              |
| modifiers        | 1     | Bitmask: Ctrl, Shift, Alt, Meta, etc.   |
+------------------+-------+------------------------------------------+
```

**Key Code Convention**: USB HID Usage IDs (page 0x07) are used as the canonical cross-platform key representation. The master translates Windows virtual key codes to HID Usage IDs before transmission. Each client translates HID Usage IDs to platform-specific codes for injection.

**Modifier Bitmask**:
- Bit 0: Left Ctrl
- Bit 1: Right Ctrl
- Bit 2: Left Shift
- Bit 3: Right Shift
- Bit 4: Left Alt
- Bit 5: Right Alt
- Bit 6: Left Meta (Windows/Command/Super)
- Bit 7: Right Meta

### 4.5 MOUSE_MOVE (0x41)

```
+------------------+-------+------------------------------------------+
| Field            | Bytes | Description                              |
+------------------+-------+------------------------------------------+
| x                | 4     | Absolute X position (signed, pixels)     |
| y                | 4     | Absolute Y position (signed, pixels)     |
| delta_x          | 2     | Relative X movement (signed)             |
| delta_y          | 2     | Relative Y movement (signed)             |
+------------------+-------+------------------------------------------+
```

Mouse position is in the client's local coordinate space (0,0 = top-left of primary monitor).

### 4.6 MOUSE_BUTTON (0x42)

```
+------------------+-------+------------------------------------------+
| Field            | Bytes | Description                              |
+------------------+-------+------------------------------------------+
| button           | 1     | 0x01=left, 0x02=right, 0x03=middle,     |
|                  |       | 0x04=button4, 0x05=button5               |
| event_type       | 1     | 0x01=press, 0x02=release                |
| x                | 4     | X position at time of click (absolute)  |
| y                | 4     | Y position at time of click (absolute)  |
+------------------+-------+------------------------------------------+
```

### 4.7 MOUSE_SCROLL (0x43)

```
+------------------+-------+------------------------------------------+
| Field            | Bytes | Description                              |
+------------------+-------+------------------------------------------+
| delta_x          | 2     | Horizontal scroll amount (signed)        |
| delta_y          | 2     | Vertical scroll amount (signed)          |
| x                | 4     | X position of cursor (absolute)          |
| y                | 4     | Y position of cursor (absolute)          |
+------------------+-------+------------------------------------------+
```

**Scroll unit**: 1 unit = 1/120th of a "notch" (matching Windows WHEEL_DELTA convention).

### 4.8 ANNOUNCE (0x80)

```
+------------------+-------+------------------------------------------+
| Field            | Bytes | Description                              |
+------------------+-------+------------------------------------------+
| client_id        | 16    | UUID v4 of the client                    |
| platform_id      | 1     | Same as HELLO                            |
| control_port     | 2     | TCP port client is listening on          |
| client_name_len  | 2     | Length of name                           |
| client_name      | var   | Human-readable hostname                  |
+------------------+-------+------------------------------------------+
```

### 4.9 CLIPBOARD_DATA (0x0B)

```
+------------------+-------+------------------------------------------+
| Field            | Bytes | Description                              |
+------------------+-------+------------------------------------------+
| format           | 1     | 0x01=UTF-8 text, 0x02=HTML, 0x03=image  |
| data_length      | 4     | Length of clipboard data                 |
| data             | var   | Raw clipboard content                    |
+------------------+-------+------------------------------------------+
```

Large clipboard data (>64KB) is fragmented across multiple CLIPBOARD_DATA messages with a continuation flag in the Reserved header field (bit 0: 1=more fragments, 0=last fragment).

---

## 5. Connection Lifecycle

### 5.1 Initial Connection Sequence

```
Client                                    Master
  |                                          |
  |--- ANNOUNCE (UDP broadcast) ------------>|
  |<-- ANNOUNCE_RESPONSE (UDP unicast) ------|
  |                                          |
  |--- TCP connect to master:24800 -------->|
  |--- TLS handshake (HELLO) -------------->|
  |<-- TLS handshake (HELLO_ACK) -----------|
  |                                          |
  | [If new/unpaired client:]               |
  |<-- PAIRING_REQUEST (PIN displayed) -----|
  |--- PAIRING_RESPONSE (PIN entered) ----->|
  |                                          |
  |--- SCREEN_INFO ------------------------>|
  |<-- SCREEN_INFO_ACK ---------------------|
  |                                          |
  | [DTLS handshake on UDP:24801]           |
  |<-- Input events (UDP/DTLS) -------------|
  |                                          |
```

### 5.2 Reconnection Behavior

- If the control channel drops, the client attempts reconnection with exponential backoff: 1s, 2s, 4s, 8s, 16s, cap at 30s.
- If the input channel drops while control is alive, a new DTLS session is negotiated on the existing control channel session.
- If the master is not found after 120 seconds of reconnection attempts, the client enters "discovery" mode and begins broadcasting ANNOUNCE messages again.

### 5.3 Graceful Disconnect

- Either side sends DISCONNECT message on the control channel.
- Receiving side flushes any pending input events, stops the input channel, then closes the TCP connection.
- Master marks the client as disconnected in the layout (greyed out) but retains its position.

---

## 6. Security Model

### 6.1 Pairing Process

1. When an unpaired client connects, the master generates a 6-digit numeric PIN.
2. The PIN is displayed in the master UI and must be entered on the client within 60 seconds.
3. On success, the client's certificate is pinned on the master (stored by client_id).
4. On failure, 3 attempts are allowed before a 60-second lockout.

### 6.2 Session Token Usage

The session token from HELLO_ACK is included in the DTLS client hello as a pre-shared key identifier, binding the UDP input channel to the authenticated TCP control session.

### 6.3 Key Material Storage

| Location | Storage Method |
|----------|---------------|
| Windows Master | Windows DPAPI encrypted file in %APPDATA%\KVMOverIP |
| Windows Client | Windows DPAPI encrypted file in %APPDATA%\KVMOverIP |
| Linux Client | AES-256 encrypted file, key derived from OS keyring |
| macOS Client | macOS Keychain |
| Web Client | Session only; no persistent key storage |

---

## 7. Error Codes

| Code | Name | Description |
|------|------|-------------|
| 0x01 | PROTOCOL_VERSION_MISMATCH | Client version incompatible |
| 0x02 | AUTHENTICATION_FAILED | Invalid session token |
| 0x03 | PAIRING_REQUIRED | Client not paired with master |
| 0x04 | PAIRING_FAILED | PIN incorrect or timed out |
| 0x05 | TOO_MANY_CLIENTS | Master client limit reached |
| 0x06 | RATE_LIMITED | Too many connection attempts |
| 0x07 | INTERNAL_ERROR | Unexpected server error |
| 0x08 | INVALID_MESSAGE | Malformed message received |
