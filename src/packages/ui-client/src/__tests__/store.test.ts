/**
 * Tests for the `useClientStore` Zustand store.
 *
 * # What these tests verify
 *
 * Each action in the store (`setStatus`, `setSettings`, etc.) is tested to
 * confirm it writes the expected value to the store and that the initial
 * state is correct.
 *
 * # Test isolation
 *
 * The `afterEach` hook resets the store to its initial state after every test.
 * Without this reset, state written by one test would leak into the next test,
 * making tests order-dependent and unreliable.
 *
 * Zustand exposes `useClientStore.setState(...)` for direct state manipulation
 * in tests — this bypasses actions and lets us set up specific starting
 * conditions quickly.
 */

import { useClientStore } from "../store";
import type { ClientStatusDto, ClientSettingsDto } from "../types";

// ── Test isolation ─────────────────────────────────────────────────────────────

// Reset the store to defaults after each test to prevent state leakage
afterEach(() => {
  useClientStore.setState({
    status: null,
    settings: { masterAddress: "", clientName: "" },
    isLoading: false,
    lastError: null,
  });
});

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("useClientStore", () => {
  test("setStatus stores the status", () => {
    // Arrange
    const status: ClientStatusDto = {
      connectionStatus: "Active",
      masterAddress: "192.168.1.1",
      clientName: "laptop",
      monitorCount: 1,
    };

    // Act
    useClientStore.getState().setStatus(status);

    // Assert
    expect(useClientStore.getState().status).toEqual(status);
  });

  test("setSettings stores the settings", () => {
    // Arrange
    const settings: ClientSettingsDto = {
      masterAddress: "10.0.0.1",
      clientName: "work-pc",
    };

    // Act
    useClientStore.getState().setSettings(settings);

    // Assert
    expect(useClientStore.getState().settings).toEqual(settings);
  });

  test("setLoading updates the loading flag", () => {
    // Act
    useClientStore.getState().setLoading(true);

    // Assert
    expect(useClientStore.getState().isLoading).toBe(true);
  });

  test("setLastError stores the error string", () => {
    // Act
    useClientStore.getState().setLastError("test error");

    // Assert
    expect(useClientStore.getState().lastError).toBe("test error");
  });

  test("setLastError clears the error when null is passed", () => {
    // Arrange — seed an error
    useClientStore.getState().setLastError("some error");

    // Act — clear it
    useClientStore.getState().setLastError(null);

    // Assert
    expect(useClientStore.getState().lastError).toBeNull();
  });

  test("initial status is null", () => {
    // Assert — the store should start with no status data
    expect(useClientStore.getState().status).toBeNull();
  });

  test("initial loading flag is false", () => {
    // Assert — the store should not start in a loading state
    expect(useClientStore.getState().isLoading).toBe(false);
  });
});
