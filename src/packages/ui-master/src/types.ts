/**
 * TypeScript types matching the Rust DTOs exposed by kvm-master Tauri commands.
 *
 * These types are the contract between the React UI and the Rust backend.
 * Any change to the Rust DTO structs must be mirrored here.
 */

// ── Client DTOs ───────────────────────────────────────────────────────────────

/** A connected or discovered KVM client. */
export interface ClientDto {
  /** UUID string identifying the client. */
  clientId: string;
  /** Human-readable display name. */
  name: string;
  /** Serialised ConnectionState enum value. */
  connectionState: string;
  /** Round-trip latency in milliseconds. */
  latencyMs: number;
  /** Input events received per second. */
  eventsPerSecond: number;
}

// ── Layout DTOs ───────────────────────────────────────────────────────────────

/** Positioned layout entry for a single client screen. */
export interface ClientLayoutDto {
  /** UUID string identifying the client. */
  clientId: string;
  /** Human-readable display name. */
  name: string;
  /** X offset in pixels relative to master top-left. */
  xOffset: number;
  /** Y offset in pixels relative to master top-left. */
  yOffset: number;
  /** Screen width in pixels. */
  width: number;
  /** Screen height in pixels. */
  height: number;
}

// ── Network DTOs ──────────────────────────────────────────────────────────────

/** Network configuration. */
export interface NetworkConfigDto {
  controlPort: number;
  inputPort: number;
  discoveryPort: number;
  bindAddress: string;
}

// ── Command result wrapper ────────────────────────────────────────────────────

/** Unified result type returned by all Tauri commands. */
export interface CommandResult<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}
