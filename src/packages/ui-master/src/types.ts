/**
 * TypeScript types matching the Rust DTOs exposed by kvm-master Tauri commands.
 *
 * These types are the contract between the React UI and the Rust backend.
 * Any change to the Rust DTO structs must be mirrored here.
 *
 * # What is a DTO? (for beginners)
 *
 * A DTO (Data Transfer Object) is a plain data structure used to carry
 * information across a boundary — in this case, the boundary between the
 * Rust backend and the React frontend.  DTOs contain only data (no methods),
 * so they can be serialized to JSON and deserialized on the other side.
 *
 * Tauri automatically converts Rust structs (that `#[derive(Serialize)]`) to
 * JSON when a command returns them, and converts JSON back to Rust structs
 * when a command receives them.  The TypeScript interfaces below describe the
 * JSON shape that Tauri produces.
 *
 * # Naming convention
 *
 * TypeScript uses camelCase field names; the Rust structs use snake_case.
 * Tauri automatically converts `snake_case` to `camelCase` during
 * serialization, so `client_id` in Rust becomes `clientId` in TypeScript.
 */

// ── Client DTOs ───────────────────────────────────────────────────────────────

/**
 * A connected or discovered KVM client.
 *
 * Mirrors the Rust `ClientDto` struct in `kvm-master/src/infrastructure/ui_bridge/mod.rs`.
 * This is a snapshot of one entry in the `ClientRegistry`.
 */
export interface ClientDto {
  /** UUID string (e.g. "550e8400-e29b-41d4-a716-446655440000") identifying the client. */
  clientId: string;
  /** Human-readable display name sent in the Hello handshake (e.g. "jerry-laptop"). */
  name: string;
  /**
   * Serialised `ConnectionState` enum value (e.g. "Connected", "Paired", "Disconnected").
   * This is the Debug representation of the Rust enum variant.
   */
  connectionState: string;
  /** Round-trip latency in milliseconds, measured via Ping/Pong messages. */
  latencyMs: number;
  /** Input events received per second (keyboard + mouse events combined). */
  eventsPerSecond: number;
}

// ── Layout DTOs ───────────────────────────────────────────────────────────────

/**
 * Positioned layout entry for a single client screen.
 *
 * The x/y offsets are in **virtual screen pixels** relative to the top-left of
 * the master monitor.  The master monitor is always at (0, 0).  A client
 * positioned to the right of the master at the same height would have
 * `xOffset = masterWidth, yOffset = 0`.
 *
 * Mirrors the Rust `ClientLayoutDto` struct in `kvm-master/src/infrastructure/ui_bridge/mod.rs`.
 */
export interface ClientLayoutDto {
  /** UUID string identifying the client (matches `ClientDto.clientId`). */
  clientId: string;
  /** Human-readable display name (shown as the tile label in the layout editor). */
  name: string;
  /** X offset in virtual screen pixels relative to the master's top-left corner. */
  xOffset: number;
  /** Y offset in virtual screen pixels relative to the master's top-left corner. */
  yOffset: number;
  /** Screen width in pixels as reported by the client's OS. */
  width: number;
  /** Screen height in pixels as reported by the client's OS. */
  height: number;
}

// ── Network DTOs ──────────────────────────────────────────────────────────────

/**
 * Network port configuration for the master.
 *
 * Mirrors the Rust `NetworkConfigDto` in `kvm-master/src/infrastructure/ui_bridge/mod.rs`.
 *
 * Default ports:
 * - Control: 24800 (TCP — Hello/Ping/input commands)
 * - Input: 24801 (UDP — high-frequency mouse move events)
 * - Discovery: 24802 (UDP broadcast — automatic client discovery)
 */
export interface NetworkConfigDto {
  /** TCP port for the control channel (client connections, key/mouse commands). */
  controlPort: number;
  /** UDP port for high-frequency input events (mouse moves). */
  inputPort: number;
  /** UDP broadcast port used for automatic LAN discovery. */
  discoveryPort: number;
  /** IP address the master binds to (e.g. "0.0.0.0" to listen on all interfaces). */
  bindAddress: string;
}

// ── Command result wrapper ────────────────────────────────────────────────────

/**
 * Unified result type returned by all Tauri commands.
 *
 * Every command in `kvm-master/src/infrastructure/ui_bridge/mod.rs` returns
 * `CommandResult<T>` so that error handling in TypeScript is consistent.
 *
 * Usage:
 * ```ts
 * const result = await invoke<CommandResult<ClientDto[]>>("get_clients");
 * if (!result.success) throw new Error(result.error ?? "unknown error");
 * return result.data!;
 * ```
 */
export interface CommandResult<T> {
  /** `true` if the command completed successfully; `false` on error. */
  success: boolean;
  /** The command's return value.  Present when `success` is `true`, `null` otherwise. */
  data: T | null;
  /** Human-readable error description.  Present when `success` is `false`, `null` otherwise. */
  error: string | null;
}
