/**
 * Tauri IPC helpers for invoking kvm-master backend commands.
 *
 * Each exported function calls `invoke()` to talk to the corresponding
 * `pub async fn` registered in
 * `kvm-master/src/infrastructure/ui_bridge/mod.rs`.
 *
 * # How invoke() works (for beginners)
 *
 * `invoke` is Tauri's way of calling a Rust function from JavaScript/TypeScript.
 * It serializes the arguments to JSON, sends them through the IPC bridge to the
 * Rust backend, and returns a Promise that resolves to the deserialized return
 * value when the Rust function completes.
 *
 * ```ts
 * // TypeScript (this file)
 * const result = await invoke<CommandResult<ClientDto[]>>("get_clients");
 *
 * // Rust (kvm-master/src/infrastructure/ui_bridge/mod.rs)
 * #[tauri::command]
 * pub async fn get_clients(state: State<AppState>) -> CommandResult<Vec<ClientDto>> { ... }
 * ```
 *
 * # Error handling pattern
 *
 * All commands return `CommandResult<T>`.  Each helper function unwraps the
 * result and throws a JavaScript `Error` if `success` is `false`.  This lets
 * callers use standard `try/catch` without inspecting the wrapper object.
 *
 * @module api
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  ClientDto,
  ClientLayoutDto,
  CommandResult,
  NetworkConfigDto,
} from "./types";

// ── Clients ───────────────────────────────────────────────────────────────────

/**
 * Fetches all currently registered clients from the master's `ClientRegistry`.
 *
 * Called by `useClients()` every 2 seconds to keep the UI's client list
 * up-to-date as clients connect and disconnect.
 *
 * @returns Resolved list of clients on success.
 * @throws An `Error` with the backend error message if the command fails.
 */
export async function getClients(): Promise<ClientDto[]> {
  const result = await invoke<CommandResult<ClientDto[]>>("get_clients");
  if (!result.success || result.data === null) {
    throw new Error(result.error ?? "get_clients failed");
  }
  return result.data;
}

// ── Layout ────────────────────────────────────────────────────────────────────

/**
 * Fetches the current virtual screen layout from the master.
 *
 * The layout describes the x/y positions of each client's screen relative to
 * the master's top-left corner.  Used by `LayoutEditor` to render the
 * drag-and-drop canvas.
 *
 * @returns Array of positioned layout entries, one per connected client.
 * @throws An `Error` if the backend call fails.
 */
export async function getLayout(): Promise<ClientLayoutDto[]> {
  const result = await invoke<CommandResult<ClientLayoutDto[]>>("get_layout");
  if (!result.success || result.data === null) {
    throw new Error(result.error ?? "get_layout failed");
  }
  return result.data;
}

/**
 * Applies and persists a new layout configuration to the master.
 *
 * The Rust backend validates the geometry (checks for overlapping screens,
 * unreachable positions, etc.) before writing the new layout to the config
 * file on disk.  If validation fails the Promise rejects with an error.
 *
 * @param clients - The complete new layout to apply.  All client positions
 *   are replaced atomically; there is no partial update.
 * @throws An `Error` if the layout is invalid or the backend call fails.
 */
export async function updateLayout(clients: ClientLayoutDto[]): Promise<void> {
  const result = await invoke<CommandResult<null>>("update_layout", {
    clients,
  });
  if (!result.success) {
    throw new Error(result.error ?? "update_layout failed");
  }
}

// ── Network ───────────────────────────────────────────────────────────────────

/**
 * Returns the current network port configuration from the master.
 *
 * Used by the settings panel to populate the port number fields.
 *
 * @returns The active network config (ports + bind address).
 * @throws An `Error` if the backend call fails.
 */
export async function getNetworkConfig(): Promise<NetworkConfigDto> {
  const result =
    await invoke<CommandResult<NetworkConfigDto>>("get_network_config");
  if (!result.success || result.data === null) {
    throw new Error(result.error ?? "get_network_config failed");
  }
  return result.data;
}

/**
 * Applies and persists a new network configuration to the master.
 *
 * Note: changing ports requires restarting the network listeners; in the
 * current implementation the changes take effect after the master process
 * is restarted.
 *
 * @param network - New port and address settings to apply.
 * @throws An `Error` if the configuration is invalid or the backend call fails.
 */
export async function updateNetworkConfig(
  network: NetworkConfigDto
): Promise<void> {
  const result = await invoke<CommandResult<null>>("update_network_config", {
    network,
  });
  if (!result.success) {
    throw new Error(result.error ?? "update_network_config failed");
  }
}

// ── Sharing ───────────────────────────────────────────────────────────────────

/**
 * Returns whether input sharing is currently active.
 *
 * When sharing is active, keyboard and mouse events captured on the master
 * machine are routed to the currently active client.  When inactive, all
 * input stays on the master (the low-level hooks are still installed but
 * `SUPPRESS_FLAG` is never set and events pass through).
 *
 * @returns `true` if input sharing is enabled, `false` otherwise.
 */
export async function getSharingEnabled(): Promise<boolean> {
  const result = await invoke<CommandResult<boolean>>("get_sharing_enabled");
  if (!result.success || result.data === null) {
    return false;
  }
  return result.data;
}
