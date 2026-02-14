/**
 * Tests for the root App component in kvm-client UI.
 *
 * # What these tests verify
 *
 * - App renders without crashing and contains the expected DOM sections.
 * - The initial data load (`Promise.all([getClientStatus, getClientSettings])`)
 *   runs on mount and writes results into the Zustand store.
 * - Errors from the backend are stored in the global error field.
 *
 * # Test setup
 *
 * `mockInvoke` intercepts all `invoke(command)` calls that the component
 * makes via the Tauri IPC.  We provide a default implementation in `beforeEach`
 * that returns successful responses for both commands App calls on mount.
 *
 * Each test can override `mockInvoke` to simulate different scenarios
 * (e.g., a failed fetch).
 *
 * # `waitFor`
 *
 * Because the data load is asynchronous (it calls `invoke`, which returns a
 * Promise), assertions about the store state must be wrapped in `waitFor`.
 * `waitFor` retries the assertion until it passes or times out.
 */

import React from "react";
import { render, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom";
import App from "../App";
import { useClientStore } from "../store";
import { invoke } from "@tauri-apps/api/core";
import type { ClientStatusDto, ClientSettingsDto } from "../types";

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

// ── Sample data ────────────────────────────────────────────────────────────────

const sampleStatus: ClientStatusDto = {
  connectionStatus: "Active",
  masterAddress: "192.168.1.100",
  clientName: "test-client",
  monitorCount: 1,
};

const sampleSettings: ClientSettingsDto = {
  masterAddress: "192.168.1.100",
  clientName: "test-client",
};

// ── Setup / teardown ───────────────────────────────────────────────────────────

beforeEach(() => {
  // Default mock: all IPC calls succeed with the sample data.
  // Individual tests can override this using mockInvoke.mockImplementation().
  mockInvoke.mockImplementation((cmd) => {
    if (cmd === "get_client_status") {
      return Promise.resolve({ success: true, data: sampleStatus, error: null });
    }
    if (cmd === "get_client_settings") {
      return Promise.resolve({
        success: true,
        data: sampleSettings,
        error: null,
      });
    }
    return Promise.resolve({ success: true, data: null, error: null });
  });
});

afterEach(() => {
  // Reset the store between tests to prevent state leakage
  useClientStore.setState({
    status: null,
    settings: { masterAddress: "", clientName: "" },
    isLoading: false,
    lastError: null,
  });
  mockInvoke.mockReset();
});

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("App (ui-client)", () => {
  test("renders the application root element", () => {
    render(<App />);
    expect(screen.getByTestId("client-app-root")).toBeInTheDocument();
  });

  test("renders the application title heading", () => {
    render(<App />);
    expect(
      screen.getByRole("heading", { name: /kvm-over-ip client/i })
    ).toBeInTheDocument();
  });

  test("renders the connection status section", () => {
    render(<App />);
    // The section has aria-label="Connection status" which creates a landmark region
    expect(
      screen.getByRole("region", { name: /connection status/i })
    ).toBeInTheDocument();
  });

  test("renders the client settings section", () => {
    render(<App />);
    // The section has aria-label="Client settings" which creates a landmark region
    expect(
      screen.getByRole("region", { name: /client settings/i })
    ).toBeInTheDocument();
  });

  test("loads status and settings into the store on mount", async () => {
    render(<App />);

    // waitFor retries until both values appear in the store
    await waitFor(() => {
      const state = useClientStore.getState();
      expect(state.status).toEqual(sampleStatus);
      expect(state.settings).toEqual(sampleSettings);
    });
  });

  test("clears loading flag after initial data load", async () => {
    render(<App />);

    await waitFor(() => {
      expect(useClientStore.getState().isLoading).toBe(false);
    });
  });

  test("stores error message in store when initial load fails", async () => {
    // Override the default mock to simulate a backend failure
    mockInvoke.mockRejectedValue(new Error("backend unreachable"));

    render(<App />);

    await waitFor(() => {
      expect(useClientStore.getState().lastError).toBe("backend unreachable");
    });
  });
});
