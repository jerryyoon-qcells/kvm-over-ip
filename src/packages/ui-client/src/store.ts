/**
 * Global application state store for the client UI using Zustand.
 */

import { create } from "zustand";
import type { ClientStatusDto, ClientSettingsDto } from "./types";

export interface ClientState {
  status: ClientStatusDto | null;
  settings: ClientSettingsDto;
  isLoading: boolean;
  lastError: string | null;

  setStatus: (status: ClientStatusDto) => void;
  setSettings: (settings: ClientSettingsDto) => void;
  setLoading: (loading: boolean) => void;
  setLastError: (error: string | null) => void;
}

export const useClientStore = create<ClientState>((set) => ({
  status: null,
  settings: { masterAddress: "", clientName: "" },
  isLoading: false,
  lastError: null,

  setStatus: (status) => set({ status }),
  setSettings: (settings) => set({ settings }),
  setLoading: (loading) => set({ isLoading: loading }),
  setLastError: (error) => set({ lastError: error }),
}));
