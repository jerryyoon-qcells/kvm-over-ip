/**
 * Tests for the ui-client API module.
 *
 * # What these tests verify
 *
 * Each helper function in `api.ts` wraps a Tauri `invoke()` call and unwraps
 * the `CommandResult<T>` envelope.  These tests verify that each helper:
 * - Calls `invoke` with the correct command name (and arguments, where applicable).
 * - Returns the `data` field from a successful `CommandResult`.
 * - Throws an `Error` containing the `error` field from a failed `CommandResult`.
 * - Uses a sensible default error message when the `error` field is `null`.
 * - Falls back gracefully for commands that do not throw on failure
 *   (e.g., `getMonitorCount` returns 0 instead of throwing).
 *
 * # `ok` / `fail` helpers
 *
 * These builder functions construct `CommandResult<T>` objects matching
 * the wire format returned by the Rust backend.  Using them keeps test
 * arrangements concise and consistent.
 *
 * # Mock setup
 *
 * `mockInvoke.mockReset()` in `beforeEach` clears any return values and call
 * history set by the previous test, ensuring each test starts fresh.
 */

import { invoke } from "@tauri-apps/api/core";
import {
  getClientStatus,
  getClientSettings,
  updateClientSettings,
  getMonitorCount,
} from "../api";
import type {
  ClientStatusDto,
  ClientSettingsDto,
  CommandResult,
} from "../types";

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

// Reset mock between tests to avoid cross-test contamination
beforeEach(() => {
  mockInvoke.mockReset();
});

// ── Helpers ────────────────────────────────────────────────────────────────────

/**
 * Builds a successful `CommandResult<T>` containing `data`.
 */
function ok<T>(data: T): CommandResult<T> {
  return { success: true, data, error: null };
}

/**
 * Builds a failed `CommandResult` containing an error message.
 */
function fail(error: string): CommandResult<never> {
  return { success: false, data: null, error };
}

// ── Sample data ────────────────────────────────────────────────────────────────

const sampleStatus: ClientStatusDto = {
  connectionStatus: "Active",
  masterAddress: "192.168.1.1",
  clientName: "laptop",
  monitorCount: 2,
};

const sampleSettings: ClientSettingsDto = {
  masterAddress: "192.168.1.1",
  clientName: "laptop",
};

// ── getClientStatus ────────────────────────────────────────────────────────────

describe("getClientStatus", () => {
  test("returns status on success", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(ok(sampleStatus));

    // Act
    const result = await getClientStatus();

    // Assert — the returned data matches the mocked backend response
    expect(result).toEqual(sampleStatus);
    // Assert — the correct Tauri command name was used
    expect(mockInvoke).toHaveBeenCalledWith("get_client_status");
  });

  test("throws when backend returns failure", async () => {
    // Arrange — simulate a backend error
    mockInvoke.mockResolvedValue(fail("not connected"));

    // Act / Assert — the error field is re-thrown as a JavaScript Error
    await expect(getClientStatus()).rejects.toThrow("not connected");
  });

  test("throws with default message when error field is null", async () => {
    // Arrange — success:false but no error message provided
    mockInvoke.mockResolvedValue({ success: false, data: null, error: null });

    // Act / Assert — falls back to the default message
    await expect(getClientStatus()).rejects.toThrow("get_client_status failed");
  });
});

// ── getClientSettings ──────────────────────────────────────────────────────────

describe("getClientSettings", () => {
  test("returns settings on success", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(ok(sampleSettings));

    // Act
    const result = await getClientSettings();

    // Assert
    expect(result).toEqual(sampleSettings);
    expect(mockInvoke).toHaveBeenCalledWith("get_client_settings");
  });

  test("throws when backend returns failure", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(fail("settings unavailable"));

    // Act / Assert
    await expect(getClientSettings()).rejects.toThrow("settings unavailable");
  });

  test("throws with default message when error field is null", async () => {
    // Arrange
    mockInvoke.mockResolvedValue({ success: false, data: null, error: null });

    // Act / Assert
    await expect(getClientSettings()).rejects.toThrow(
      "get_client_settings failed"
    );
  });
});

// ── updateClientSettings ───────────────────────────────────────────────────────

describe("updateClientSettings", () => {
  test("resolves silently on success", async () => {
    // Arrange — backend returns success with no data (void command)
    mockInvoke.mockResolvedValue(ok(null));

    // Act / Assert — resolves to undefined (no return value on success)
    await expect(
      updateClientSettings(sampleSettings)
    ).resolves.toBeUndefined();
    // Assert — called with both the command name AND the settings payload
    expect(mockInvoke).toHaveBeenCalledWith("update_client_settings", {
      settings: sampleSettings,
    });
  });

  test("throws when backend returns failure", async () => {
    // Arrange — simulate a validation error from the Rust side
    mockInvoke.mockResolvedValue(fail("validation error"));

    // Act / Assert
    await expect(updateClientSettings(sampleSettings)).rejects.toThrow(
      "validation error"
    );
  });

  test("throws with default message when error field is null", async () => {
    // Arrange
    mockInvoke.mockResolvedValue({ success: false, data: null, error: null });

    // Act / Assert
    await expect(updateClientSettings(sampleSettings)).rejects.toThrow(
      "update_client_settings failed"
    );
  });
});

// ── getMonitorCount ────────────────────────────────────────────────────────────

describe("getMonitorCount", () => {
  test("returns monitor count on success", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(ok(3));

    // Act
    const result = await getMonitorCount();

    // Assert
    expect(result).toBe(3);
    expect(mockInvoke).toHaveBeenCalledWith("get_monitor_count");
  });

  test("returns 0 when backend reports failure", async () => {
    // Arrange — graceful degradation: `getMonitorCount` does not throw on failure
    mockInvoke.mockResolvedValue(fail("monitor detection unavailable"));

    // Act
    const result = await getMonitorCount();

    // Assert — returns 0 instead of throwing, keeping the UI functional
    expect(result).toBe(0);
  });

  test("returns 0 when data is null", async () => {
    // Arrange
    mockInvoke.mockResolvedValue({ success: false, data: null, error: null });

    // Act
    const result = await getMonitorCount();

    // Assert
    expect(result).toBe(0);
  });
});
