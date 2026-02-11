/**
 * Tauri IPC helpers for invoking kvm-master backend commands.
 *
 * Each function matches a corresponding `pub async fn` in
 * `kvm-master/src/infrastructure/ui_bridge/mod.rs`.
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
 * Fetches all currently registered clients.
 *
 * @returns Resolved list of clients on success.
 * @throws The `error` field from CommandResult if the backend reports failure.
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
 * Fetches the current virtual screen layout.
 */
export async function getLayout(): Promise<ClientLayoutDto[]> {
  const result = await invoke<CommandResult<ClientLayoutDto[]>>("get_layout");
  if (!result.success || result.data === null) {
    throw new Error(result.error ?? "get_layout failed");
  }
  return result.data;
}

/**
 * Applies and persists a new layout configuration.
 *
 * Validates geometry on the Rust side before writing to disk.
 *
 * @param clients - The complete new layout to apply.
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
 * Returns the current network port configuration.
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
 * Applies and persists a new network configuration.
 *
 * @param network - New port/address settings.
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
 */
export async function getSharingEnabled(): Promise<boolean> {
  const result = await invoke<CommandResult<boolean>>("get_sharing_enabled");
  if (!result.success || result.data === null) {
    return false;
  }
  return result.data;
}
