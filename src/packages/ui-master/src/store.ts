/**
 * Global application state store using Zustand.
 *
 * Provides reactive state for clients, layout, and network configuration.
 * All mutations go through store actions to maintain a single source of truth.
 *
 * # What is Zustand? (for beginners)
 *
 * Zustand is a lightweight React state management library.  It lets you define
 * a single global store that any component can subscribe to.  When the store
 * changes, only the components that read the changed slice re-render.
 *
 * A Zustand store has two parts:
 * 1. **State** — the data (e.g., `clients`, `layout`).
 * 2. **Actions** — functions that update the state (e.g., `setClients`).
 *
 * Both are defined together in the `create()` call.  Actions call the `set`
 * function provided by Zustand to produce a new state object.
 *
 * # Single source of truth
 *
 * The `useMasterStore` hook is the single source of truth for the UI.
 * Backend data fetched by hooks (`useClients`, `useLayout`) is written into
 * the store via actions.  Components read from the store — they do not
 * communicate directly with the backend.
 *
 * # Usage example
 *
 * ```ts
 * // Read from the store (re-renders when `clients` changes):
 * const clients = useMasterStore((s) => s.clients);
 *
 * // Write to the store:
 * const setClients = useMasterStore((s) => s.setClients);
 * setClients(newClientList);
 * ```
 *
 * @module store
 */

import { create } from "zustand";
import type { ClientDto, ClientLayoutDto, NetworkConfigDto } from "./types";

// ── State shape ───────────────────────────────────────────────────────────────

/**
 * The complete shape of the master application store.
 *
 * Fields are grouped by concern:
 * - **Clients** — the list of known KVM clients (polled from the backend).
 * - **Layout** — the virtual screen positioning data.
 * - **Network config** — port and address settings.
 * - **Sharing state** — whether input is being shared to a client.
 * - **Error state** — the most recent error for display in the status bar.
 * - **Actions** — functions that update each piece of state.
 */
export interface MasterState {
  // ── Clients ────────────────────────────────────────────────────────────────

  /** All currently known clients (discovered + connected + disconnected). */
  clients: ClientDto[];
  /**
   * `true` while the first `getClients()` call is in flight.
   * Used to show a loading indicator before the first data arrives.
   */
  isLoadingClients: boolean;

  // ── Layout ─────────────────────────────────────────────────────────────────

  /** Current virtual screen layout loaded from the backend. */
  layout: ClientLayoutDto[];
  /**
   * `true` while the initial `getLayout()` call is in flight.
   * Used to show a loading indicator in the LayoutEditor.
   */
  isLoadingLayout: boolean;

  // ── Network config ─────────────────────────────────────────────────────────

  /**
   * Current network port/address configuration.
   * `null` until `getNetworkConfig()` completes for the first time.
   */
  networkConfig: NetworkConfigDto | null;

  // ── Sharing state ──────────────────────────────────────────────────────────

  /**
   * `true` when input sharing is active (keyboard and mouse events are being
   * routed to a client machine).  Reflected in the StatusBar.
   */
  sharingEnabled: boolean;

  // ── Error state ────────────────────────────────────────────────────────────

  /**
   * The most recent error from any backend operation.
   * `null` when there are no errors.  Displayed in the StatusBar.
   */
  lastError: string | null;

  // ── Actions ────────────────────────────────────────────────────────────────

  /** Replaces the entire client list. */
  setClients: (clients: ClientDto[]) => void;
  /** Sets the loading indicator for the client list. */
  setLoadingClients: (loading: boolean) => void;
  /** Replaces the entire layout. */
  setLayout: (layout: ClientLayoutDto[]) => void;
  /** Sets the loading indicator for the layout. */
  setLoadingLayout: (loading: boolean) => void;
  /**
   * Updates a single client's layout entry (identified by `clientId`).
   * All other entries remain unchanged.
   */
  updateClientLayout: (updated: ClientLayoutDto) => void;
  /** Replaces the network configuration. */
  setNetworkConfig: (config: NetworkConfigDto) => void;
  /** Sets the sharing enabled flag. */
  setSharingEnabled: (enabled: boolean) => void;
  /**
   * Sets (or clears) the last error message.
   * Pass `null` to clear any existing error.
   */
  setLastError: (error: string | null) => void;
}

// ── Store ─────────────────────────────────────────────────────────────────────

/**
 * The Zustand store for the kvm-master UI.
 *
 * Import this hook in any component or hook that needs to read or write
 * application state:
 *
 * ```ts
 * import { useMasterStore } from "../store";
 *
 * const clients = useMasterStore((s) => s.clients);
 * ```
 *
 * The selector function `(s) => s.clients` tells Zustand which slice of state
 * this component cares about.  The component only re-renders when that slice
 * changes, not when any other field changes.
 */
export const useMasterStore = create<MasterState>((set) => ({
  // Initial state — all collections empty, loading flags off
  clients: [],
  isLoadingClients: false,
  layout: [],
  isLoadingLayout: false,
  networkConfig: null,
  sharingEnabled: false,
  lastError: null,

  // Actions — each calls `set` with a partial state update
  setClients: (clients) => set({ clients }),
  setLoadingClients: (loading) => set({ isLoadingClients: loading }),
  setLayout: (layout) => set({ layout }),
  setLoadingLayout: (loading) => set({ isLoadingLayout: loading }),

  updateClientLayout: (updated) =>
    set((state) => ({
      // Replace only the matching entry; leave all others unchanged
      layout: state.layout.map((entry) =>
        entry.clientId === updated.clientId ? updated : entry
      ),
    })),

  setNetworkConfig: (config) => set({ networkConfig: config }),
  setSharingEnabled: (enabled) => set({ sharingEnabled: enabled }),
  setLastError: (error) => set({ lastError: error }),
}));
