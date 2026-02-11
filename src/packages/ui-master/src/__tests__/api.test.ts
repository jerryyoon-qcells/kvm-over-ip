/**
 * Tests for the API module – verifies that each helper correctly
 * processes CommandResult responses from the mocked invoke function.
 */

// The @tauri-apps/api/core module is auto-mocked via
// src/__mocks__/@tauri-apps/api/core.ts
import { invoke } from "@tauri-apps/api/core";
import {
  getClients,
  getLayout,
  updateLayout,
  getNetworkConfig,
  updateNetworkConfig,
  getSharingEnabled,
} from "../api";
import type { ClientDto, ClientLayoutDto, CommandResult, NetworkConfigDto } from "../types";

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

const sampleClient: ClientDto = {
  clientId: "aaaa-aaaa",
  name: "dev-linux",
  connectionState: "Connected",
  latencyMs: 2.5,
  eventsPerSecond: 60,
};

const sampleLayout: ClientLayoutDto = {
  clientId: "aaaa-aaaa",
  name: "dev-linux",
  xOffset: 1920,
  yOffset: 0,
  width: 1920,
  height: 1080,
};

const sampleNetwork: NetworkConfigDto = {
  controlPort: 24800,
  inputPort: 24801,
  discoveryPort: 24802,
  bindAddress: "0.0.0.0",
};

// ── getClients ────────────────────────────────────────────────────────────────

describe("getClients", () => {
  test("returns client list on success", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(ok([sampleClient]));

    // Act
    const result = await getClients();

    // Assert
    expect(result).toEqual([sampleClient]);
    expect(mockInvoke).toHaveBeenCalledWith("get_clients");
  });

  test("throws when backend returns failure", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(fail("network error"));

    // Act / Assert
    await expect(getClients()).rejects.toThrow("network error");
  });
});

// ── getLayout ─────────────────────────────────────────────────────────────────

describe("getLayout", () => {
  test("returns layout list on success", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(ok([sampleLayout]));

    // Act
    const result = await getLayout();

    // Assert
    expect(result).toEqual([sampleLayout]);
  });

  test("throws on backend failure", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(fail("persistence error"));

    // Act / Assert
    await expect(getLayout()).rejects.toThrow("persistence error");
  });
});

// ── updateLayout ──────────────────────────────────────────────────────────────

describe("updateLayout", () => {
  test("resolves silently on success", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(ok(null));

    // Act
    await expect(updateLayout([sampleLayout])).resolves.toBeUndefined();
    expect(mockInvoke).toHaveBeenCalledWith("update_layout", {
      clients: [sampleLayout],
    });
  });

  test("throws on backend failure", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(fail("overlapping screens"));

    // Act / Assert
    await expect(updateLayout([sampleLayout])).rejects.toThrow(
      "overlapping screens"
    );
  });
});

// ── getNetworkConfig ──────────────────────────────────────────────────────────

describe("getNetworkConfig", () => {
  test("returns config on success", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(ok(sampleNetwork));

    // Act
    const result = await getNetworkConfig();

    // Assert
    expect(result).toEqual(sampleNetwork);
  });

  test("throws when backend returns failure", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(fail("config read error"));

    // Act / Assert
    await expect(getNetworkConfig()).rejects.toThrow("config read error");
  });

  test("throws with default message when error field is null", async () => {
    // Arrange – success:false but error field is null
    mockInvoke.mockResolvedValue({ success: false, data: null, error: null });

    // Act / Assert
    await expect(getNetworkConfig()).rejects.toThrow("get_network_config failed");
  });
});

// ── updateNetworkConfig ───────────────────────────────────────────────────────

describe("updateNetworkConfig", () => {
  test("calls the correct command with the config payload", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(ok(null));

    // Act
    await updateNetworkConfig(sampleNetwork);

    // Assert
    expect(mockInvoke).toHaveBeenCalledWith("update_network_config", {
      network: sampleNetwork,
    });
  });

  test("throws when backend returns failure", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(fail("network write error"));

    // Act / Assert
    await expect(updateNetworkConfig(sampleNetwork)).rejects.toThrow(
      "network write error"
    );
  });

  test("throws with default message when error field is null", async () => {
    // Arrange – success:false but error field is null
    mockInvoke.mockResolvedValue({ success: false, data: null, error: null });

    // Act / Assert
    await expect(updateNetworkConfig(sampleNetwork)).rejects.toThrow(
      "update_network_config failed"
    );
  });
});

// ── getSharingEnabled ─────────────────────────────────────────────────────────

describe("getSharingEnabled", () => {
  test("returns true when sharing is active", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(ok(true));

    // Act
    const result = await getSharingEnabled();

    // Assert
    expect(result).toBe(true);
  });

  test("returns false when backend fails gracefully", async () => {
    // Arrange
    mockInvoke.mockResolvedValue(fail("not available"));

    // Act
    const result = await getSharingEnabled();

    // Assert
    expect(result).toBe(false);
  });
});
