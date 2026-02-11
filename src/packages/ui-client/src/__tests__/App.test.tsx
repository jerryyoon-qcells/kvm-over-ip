/**
 * Tests for the root App component in kvm-client UI.
 *
 * Confirms structural rendering and that the initial data load
 * (status + settings) completes and updates the store.
 */

import React from "react";
import { render, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom";
import App from "../App";
import { useClientStore } from "../store";
import { invoke } from "@tauri-apps/api/core";
import type { ClientStatusDto, ClientSettingsDto } from "../types";

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

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

beforeEach(() => {
  // Default mock: all IPC calls succeed.
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
  useClientStore.setState({
    status: null,
    settings: { masterAddress: "", clientName: "" },
    isLoading: false,
    lastError: null,
  });
  mockInvoke.mockReset();
});

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
    expect(
      screen.getByRole("region", { name: /connection status/i })
    ).toBeInTheDocument();
  });

  test("renders the client settings section", () => {
    render(<App />);
    expect(
      screen.getByRole("region", { name: /client settings/i })
    ).toBeInTheDocument();
  });

  test("loads status and settings into the store on mount", async () => {
    render(<App />);

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
    // Arrange – override to simulate failure
    mockInvoke.mockRejectedValue(new Error("backend unreachable"));

    render(<App />);

    await waitFor(() => {
      expect(useClientStore.getState().lastError).toBe("backend unreachable");
    });
  });
});
