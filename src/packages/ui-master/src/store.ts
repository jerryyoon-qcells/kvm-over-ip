/**
 * Global application state store using Zustand.
 *
 * Provides reactive state for clients, layout, and network configuration.
 * All mutations go through store actions to maintain a single source of truth.
 */

import { create } from "zustand";
import type { ClientDto, ClientLayoutDto, NetworkConfigDto } from "./types";

// ── State shape ───────────────────────────────────────────────────────────────

export interface MasterState {
  // ── Clients ────────────────────────────────────────────────────────────────
  clients: ClientDto[];
  isLoadingClients: boolean;

  // ── Layout ─────────────────────────────────────────────────────────────────
  layout: ClientLayoutDto[];
  isLoadingLayout: boolean;

  // ── Network config ─────────────────────────────────────────────────────────
  networkConfig: NetworkConfigDto | null;

  // ── Sharing state ──────────────────────────────────────────────────────────
  sharingEnabled: boolean;

  // ── Error state ────────────────────────────────────────────────────────────
  lastError: string | null;

  // ── Actions ────────────────────────────────────────────────────────────────
  setClients: (clients: ClientDto[]) => void;
  setLoadingClients: (loading: boolean) => void;
  setLayout: (layout: ClientLayoutDto[]) => void;
  setLoadingLayout: (loading: boolean) => void;
  updateClientLayout: (updated: ClientLayoutDto) => void;
  setNetworkConfig: (config: NetworkConfigDto) => void;
  setSharingEnabled: (enabled: boolean) => void;
  setLastError: (error: string | null) => void;
}

// ── Store ─────────────────────────────────────────────────────────────────────

export const useMasterStore = create<MasterState>((set) => ({
  // Initial state
  clients: [],
  isLoadingClients: false,
  layout: [],
  isLoadingLayout: false,
  networkConfig: null,
  sharingEnabled: false,
  lastError: null,

  // Actions
  setClients: (clients) => set({ clients }),
  setLoadingClients: (loading) => set({ isLoadingClients: loading }),
  setLayout: (layout) => set({ layout }),
  setLoadingLayout: (loading) => set({ isLoadingLayout: loading }),

  updateClientLayout: (updated) =>
    set((state) => ({
      layout: state.layout.map((entry) =>
        entry.clientId === updated.clientId ? updated : entry
      ),
    })),

  setNetworkConfig: (config) => set({ networkConfig: config }),
  setSharingEnabled: (enabled) => set({ sharingEnabled: enabled }),
  setLastError: (error) => set({ lastError: error }),
}));
