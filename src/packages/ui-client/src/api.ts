/**
 * Tauri IPC helpers for invoking kvm-client backend commands.
 *
 * Each exported function wraps a `invoke()` call to the corresponding
 * `pub async fn` registered in
 * `kvm-client/src/infrastructure/ui_bridge/mod.rs`.
 *
 * # Error handling convention
 *
 * All functions unwrap `CommandResult<T>` and throw a JavaScript `Error` when
 * `success` is `false`.  This allows callers to use standard `try/catch` blocks
 * without having to inspect the result wrapper object themselves.
 *
 * # Naming convention
 *
 * Tauri command names use `snake_case` (matching the Rust function names).
 * The TypeScript function wrappers use `camelCase` following JavaScript convention.
 * For example, the Rust function `get_client_status` is wrapped here as
 * `getClientStatus`.
 *
 * @module api
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  ClientStatusDto,
  ClientSettingsDto,
  CommandResult,
} from "./types";

/**
 * Fetches the current connection status snapshot from the client backend.
 *
 * Called on mount and every `STATUS_POLL_MS` milliseconds by `App` to keep
 * the `StatusDisplay` component up-to-date.
 *
 * @returns A full status snapshot including connection state, master address,
 *   client name, and monitor count.
 * @throws An `Error` with the backend error message if the command fails.
 */
export async function getClientStatus(): Promise<ClientStatusDto> {
  const result =
    await invoke<CommandResult<ClientStatusDto>>("get_client_status");
  if (!result.success || result.data === null) {
    throw new Error(result.error ?? "get_client_status failed");
  }
  return result.data;
}

/**
 * Fetches the current client settings (master address and client name).
 *
 * Called once on mount to populate the Settings form fields with the
 * values currently stored in the backend.
 *
 * @returns The current editable settings.
 * @throws An `Error` if the backend call fails.
 */
export async function getClientSettings(): Promise<ClientSettingsDto> {
  const result =
    await invoke<CommandResult<ClientSettingsDto>>("get_client_settings");
  if (!result.success || result.data === null) {
    throw new Error(result.error ?? "get_client_settings failed");
  }
  return result.data;
}

/**
 * Applies new client settings submitted by the user.
 *
 * The backend validates that `clientName` is not blank before writing.
 * The new settings take effect on the next connection attempt.
 *
 * @param settings - The updated master address and client name.
 * @throws An `Error` if the settings are invalid (e.g., blank client name)
 *   or the backend call fails.
 */
export async function updateClientSettings(
  settings: ClientSettingsDto
): Promise<void> {
  const result = await invoke<CommandResult<null>>("update_client_settings", {
    settings,
  });
  if (!result.success) {
    throw new Error(result.error ?? "update_client_settings failed");
  }
}

/**
 * Returns the number of monitors detected on this client machine.
 *
 * Used to display the monitor count in `StatusDisplay`.  Returns `0` if the
 * enumeration fails (e.g., the native screen info module is not available in
 * the current build).
 *
 * @returns The number of connected monitors, or `0` on error.
 */
export async function getMonitorCount(): Promise<number> {
  const result = await invoke<CommandResult<number>>("get_monitor_count");
  if (!result.success || result.data === null) {
    // Return 0 rather than throwing so a monitor count failure does not
    // break the rest of the status display.
    return 0;
  }
  return result.data;
}
