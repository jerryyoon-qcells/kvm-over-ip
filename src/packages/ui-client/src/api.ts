/**
 * Tauri IPC helpers for invoking kvm-client backend commands.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  ClientStatusDto,
  ClientSettingsDto,
  CommandResult,
} from "./types";

export async function getClientStatus(): Promise<ClientStatusDto> {
  const result =
    await invoke<CommandResult<ClientStatusDto>>("get_client_status");
  if (!result.success || result.data === null) {
    throw new Error(result.error ?? "get_client_status failed");
  }
  return result.data;
}

export async function getClientSettings(): Promise<ClientSettingsDto> {
  const result =
    await invoke<CommandResult<ClientSettingsDto>>("get_client_settings");
  if (!result.success || result.data === null) {
    throw new Error(result.error ?? "get_client_settings failed");
  }
  return result.data;
}

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

export async function getMonitorCount(): Promise<number> {
  const result = await invoke<CommandResult<number>>("get_monitor_count");
  if (!result.success || result.data === null) {
    return 0;
  }
  return result.data;
}
