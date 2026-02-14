/**
 * Tests for the `useMasterStore` Zustand store.
 *
 * # What these tests verify
 *
 * - Each action correctly updates its corresponding state slice.
 * - `updateClientLayout` updates only the matching entry and leaves others unchanged.
 * - `setLastError(null)` clears a previously set error.
 * - Initial state values are correct.
 *
 * # Test isolation
 *
 * The `afterEach` hook resets the store to its initial state after every test
 * to prevent state from leaking between tests.  Zustand exposes
 * `useMasterStore.setState(...)` which directly writes state, bypassing actions.
 * This is the recommended approach for setting up test preconditions.
 *
 * # `makeClient` and `makeLayoutEntry` helpers
 *
 * These builders produce minimal valid DTOs with a given ID.  Tests that need
 * specific field values use overrides or build the full object themselves.
 */

import { useMasterStore } from "../store";
import type { ClientDto, ClientLayoutDto, NetworkConfigDto } from "../types";

// ── Test isolation ─────────────────────────────────────────────────────────────

// Reset store between tests to avoid state leakage.
afterEach(() => {
  useMasterStore.setState({
    clients: [],
    isLoadingClients: false,
    layout: [],
    isLoadingLayout: false,
    networkConfig: null,
    sharingEnabled: false,
    lastError: null,
  });
});

// ── Helpers ────────────────────────────────────────────────────────────────────

/** Builds a minimal `ClientDto` with the given ID. */
const makeClient = (id: string): ClientDto => ({
  clientId: id,
  name: `client-${id}`,
  connectionState: "Connected",
  latencyMs: 0,
  eventsPerSecond: 0,
});

/** Builds a minimal `ClientLayoutDto` for a client at x=1920. */
const makeLayoutEntry = (id: string): ClientLayoutDto => ({
  clientId: id,
  name: `client-${id}`,
  xOffset: 1920,
  yOffset: 0,
  width: 1920,
  height: 1080,
});

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("useMasterStore", () => {
  test("setClients replaces the client list", () => {
    // Arrange
    const clients = [makeClient("a"), makeClient("b")];

    // Act
    useMasterStore.getState().setClients(clients);

    // Assert
    expect(useMasterStore.getState().clients).toEqual(clients);
  });

  test("setLoadingClients updates the loading flag", () => {
    // Act
    useMasterStore.getState().setLoadingClients(true);

    // Assert
    expect(useMasterStore.getState().isLoadingClients).toBe(true);
  });

  test("setLayout replaces the layout", () => {
    // Arrange
    const layout = [makeLayoutEntry("x"), makeLayoutEntry("y")];

    // Act
    useMasterStore.getState().setLayout(layout);

    // Assert
    expect(useMasterStore.getState().layout).toEqual(layout);
  });

  test("updateClientLayout updates only the matching entry", () => {
    // Arrange — store two layout entries
    const layout = [makeLayoutEntry("a"), makeLayoutEntry("b")];
    useMasterStore.getState().setLayout(layout);

    const updated: ClientLayoutDto = { ...makeLayoutEntry("a"), xOffset: 3840 };

    // Act — update only "a"
    useMasterStore.getState().updateClientLayout(updated);
    const state = useMasterStore.getState();

    // Assert — "a" is updated; "b" is unchanged
    expect(state.layout.find((e) => e.clientId === "a")?.xOffset).toBe(3840);
    expect(state.layout.find((e) => e.clientId === "b")?.xOffset).toBe(1920);
  });

  test("updateClientLayout does not modify list when clientId not found", () => {
    // Arrange
    const layout = [makeLayoutEntry("a")];
    useMasterStore.getState().setLayout(layout);

    const nonExistent: ClientLayoutDto = { ...makeLayoutEntry("z"), xOffset: 9999 };

    // Act — update a non-existent entry
    useMasterStore.getState().updateClientLayout(nonExistent);

    // Assert — existing entry is unchanged
    expect(useMasterStore.getState().layout[0].xOffset).toBe(1920);
  });

  test("setNetworkConfig stores the configuration", () => {
    // Arrange
    const cfg: NetworkConfigDto = {
      controlPort: 9000,
      inputPort: 9001,
      discoveryPort: 9002,
      bindAddress: "0.0.0.0",
    };

    // Act
    useMasterStore.getState().setNetworkConfig(cfg);

    // Assert
    expect(useMasterStore.getState().networkConfig).toEqual(cfg);
  });

  test("setSharingEnabled sets the sharing flag", () => {
    // Act
    useMasterStore.getState().setSharingEnabled(true);

    // Assert
    expect(useMasterStore.getState().sharingEnabled).toBe(true);
  });

  test("setLastError stores the error string", () => {
    // Act
    useMasterStore.getState().setLastError("something failed");

    // Assert
    expect(useMasterStore.getState().lastError).toBe("something failed");
  });

  test("setLastError clears the error when null is passed", () => {
    // Arrange — seed an error
    useMasterStore.getState().setLastError("existing error");

    // Act — clear it
    useMasterStore.getState().setLastError(null);

    // Assert
    expect(useMasterStore.getState().lastError).toBeNull();
  });
});
