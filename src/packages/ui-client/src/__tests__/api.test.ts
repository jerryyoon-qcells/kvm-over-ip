/**
 * Tests for the ui-client API module.
 *
 * Verifies that each helper correctly interprets CommandResult responses
 * returned by the mocked Tauri invoke function.
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

beforeEach(() => {
  mockInvoke.mockReset();
});

// ── Helpers ───────────────────────────────────────────────────────────────────

function ok<T>(data: T): CommandResult<T> {
  return { success: true, data, error: null };
}

function fail(error: string): CommandResult<never> {
  return { success: false, data: null, error };
}

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

// ── getClientStatus ───────────────────────────────────────────────────────────

describe("getClientStatus", () => {
  test("returns status on success", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(ok(sampleStatus));

    // Act
    const result = await getClientStatus();

    // Assert
    expect(result).toEqual(sampleStatus);
    expect(mockInvoke).toHaveBeenCalledWith("get_client_status");
  });

  test("throws when backend returns failure", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(fail("not connected"));

    // Act / Assert
    await expect(getClientStatus()).rejects.toThrow("not connected");
  });

  test("throws with default message when error field is null", async () => {
    // Arrange
    mockInvoke.mockResolvedValue({ success: false, data: null, error: null });

    // Act / Assert
    await expect(getClientStatus()).rejects.toThrow("get_client_status failed");
  });
});

// ── getClientSettings ─────────────────────────────────────────────────────────

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

// ── updateClientSettings ──────────────────────────────────────────────────────

describe("updateClientSettings", () => {
  test("resolves silently on success", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(ok(null));

    // Act
    await expect(
      updateClientSettings(sampleSettings)
    ).resolves.toBeUndefined();
    expect(mockInvoke).toHaveBeenCalledWith("update_client_settings", {
      settings: sampleSettings,
    });
  });

  test("throws when backend returns failure", async () => {
    // Arrange
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

// ── getMonitorCount ───────────────────────────────────────────────────────────

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
    // Arrange – graceful degradation: returns 0 on failure
    mockInvoke.mockResolvedValue(fail("monitor detection unavailable"));

    // Act
    const result = await getMonitorCount();

    // Assert – does not throw; falls back to 0
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
