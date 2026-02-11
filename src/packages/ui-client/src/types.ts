/**
 * TypeScript types matching the Rust DTOs exposed by kvm-client Tauri commands.
 */

/** Connection status values matching the Rust ClientConnectionStatus enum. */
export type ClientConnectionStatus =
  | "Disconnected"
  | "Connecting"
  | "Connected"
  | "Active"
  | "Pairing";

/** Full client status snapshot. */
export interface ClientStatusDto {
  connectionStatus: ClientConnectionStatus;
  masterAddress: string;
  clientName: string;
  monitorCount: number;
}

/** Editable settings. */
export interface ClientSettingsDto {
  masterAddress: string;
  clientName: string;
}

/** Unified result type returned by all Tauri commands. */
export interface CommandResult<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}
