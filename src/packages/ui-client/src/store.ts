/**
 * Global application state store for the client UI using Zustand.
 *
 * Provides reactive state for the connection status, settings, loading
 * indicators, and error messages.  All mutations go through store actions
 * to maintain a single source of truth.
 *
 * # Store shape
 *
 * The client UI is simpler than the master UI — it has fewer concerns:
 * - **status** — a snapshot of the connection state, refreshed by polling.
 * - **settings** — the user-editable master address and client name.
 * - **isLoading** — `true` during the initial data fetch.
 * - **lastError** — the most recent error for display near relevant UI.
 *
 * # Usage
 *
 * ```ts
 * // Read a slice (component re-renders when this slice changes):
 * const status = useClientStore((s) => s.status);
 *
 * // Write (from an event handler or effect):
 * const setStatus = useClientStore((s) => s.setStatus);
 * setStatus(newStatus);
 * ```
 *
 * @module store
 */

import { create } from "zustand";
import type { ClientStatusDto, ClientSettingsDto } from "./types";

/**
 * The complete shape of the client application store.
 */
export interface ClientState {
  /**
   * The latest status snapshot from the backend.
   * `null` until the first `getClientStatus()` call completes.
   */
  status: ClientStatusDto | null;

  /**
   * The current client settings (master address + client name).
   * Defaults to empty strings; populated on mount by `getClientSettings()`.
   */
  settings: ClientSettingsDto;

  /**
   * `true` while the initial data load is in progress.
   * Used by `StatusDisplay` to show a loading indicator.
   */
  isLoading: boolean;

  /**
   * The most recent error message from any backend operation.
   * `null` when there are no errors.
   */
  lastError: string | null;

  // ── Actions ────────────────────────────────────────────────────────────────

  /** Replaces the status snapshot with a new one. */
  setStatus: (status: ClientStatusDto) => void;
  /** Replaces the settings with a new snapshot. */
  setSettings: (settings: ClientSettingsDto) => void;
  /** Sets the loading indicator. */
  setLoading: (loading: boolean) => void;
  /**
   * Sets (or clears) the last error message.
   * Pass `null` to clear any existing error after a successful operation.
   */
  setLastError: (error: string | null) => void;
}

/**
 * The Zustand store for the kvm-client UI.
 *
 * Import this hook in any component or hook that needs to read or write
 * application state.
 */
export const useClientStore = create<ClientState>((set) => ({
  // ── Initial state ──────────────────────────────────────────────────────────
  status: null,
  settings: { masterAddress: "", clientName: "" },
  isLoading: false,
  lastError: null,

  // ── Actions ────────────────────────────────────────────────────────────────
  setStatus: (status) => set({ status }),
  setSettings: (settings) => set({ settings }),
  setLoading: (loading) => set({ isLoading: loading }),
  setLastError: (error) => set({ lastError: error }),
}));
