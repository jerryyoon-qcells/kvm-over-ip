/**
 * TypeScript types matching the Rust DTOs exposed by kvm-client Tauri commands.
 *
 * These types are the contract between the React UI and the Rust backend.
 * Any change to the Rust DTO structs in
 * `kvm-client/src/infrastructure/ui_bridge/mod.rs` must be mirrored here.
 *
 * # How Tauri serializes enum variants
 *
 * The Rust `ClientConnectionStatus` enum is serialised using Rust's `Debug`
 * trait, which produces the variant name as a string (e.g., `Active`).
 * The TypeScript union type below enumerates all possible string values so
 * that TypeScript can catch typos and enforce exhaustive checks.
 */

/**
 * Connection status values matching the Rust `ClientConnectionStatus` enum.
 *
 * | Value          | Meaning                                                |
 * |----------------|--------------------------------------------------------|
 * | Disconnected   | Not yet attempting to connect                          |
 * | Connecting     | TCP connect to master in progress                      |
 * | Connected      | Control channel established, awaiting HelloAck/pairing |
 * | Pairing        | PIN pairing dialog is showing                          |
 * | Active         | Fully paired; receiving input events from master       |
 */
export type ClientConnectionStatus =
  | "Disconnected"
  | "Connecting"
  | "Connected"
  | "Active"
  | "Pairing";

/**
 * Full client status snapshot returned by `get_client_status`.
 *
 * Mirrors the Rust `ClientStatusDto` struct.  All fields are read-only
 * snapshots; to change settings use `ClientSettingsDto` + `updateClientSettings`.
 */
export interface ClientStatusDto {
  /** The current connection state (one of the `ClientConnectionStatus` values). */
  connectionStatus: ClientConnectionStatus;
  /** The master's IP address and port, e.g., "192.168.1.10:24800". */
  masterAddress: string;
  /** The human-readable name this client advertises to the master. */
  clientName: string;
  /** Number of monitors detected on this machine. */
  monitorCount: number;
}

/**
 * Editable client settings.
 *
 * Sent to `update_client_settings` when the user submits the Settings form.
 * Mirrors the Rust `ClientSettingsDto` struct.
 */
export interface ClientSettingsDto {
  /**
   * The master's address (IP + port).  Leave empty to enable auto-discovery
   * via UDP broadcast.
   */
  masterAddress: string;
  /**
   * The name that identifies this client in the master's client list.
   * Must not be blank.
   */
  clientName: string;
}

/**
 * Unified result type returned by all Tauri commands.
 *
 * Mirrors the Rust `ClientCommandResult<T>` struct.  Every command returns
 * this wrapper so the TypeScript side has a consistent error-handling pattern.
 *
 * Usage:
 * ```ts
 * const result = await invoke<CommandResult<ClientStatusDto>>("get_client_status");
 * if (!result.success) throw new Error(result.error ?? "unknown");
 * return result.data!;
 * ```
 */
export interface CommandResult<T> {
  /** `true` if the command succeeded; `false` on error. */
  success: boolean;
  /** The command's return value.  `null` when `success` is `false`. */
  data: T | null;
  /** Human-readable error description.  `null` when `success` is `true`. */
  error: string | null;
}
