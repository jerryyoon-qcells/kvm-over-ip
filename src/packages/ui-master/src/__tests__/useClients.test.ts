/**
 * Tests for the useClients hook.
 *
 * Verifies that the hook fetches the client list on mount, updates the
 * Zustand store, and handles errors gracefully.  Polling behaviour is
 * verified by advancing fake timers.
 */

import { renderHook, waitFor, act } from "@testing-library/react";
import "@testing-library/jest-dom";
import { useClients } from "../hooks/useClients";
import { useMasterStore } from "../store";
import { invoke } from "@tauri-apps/api/core";
import type { ClientDto } from "../types";

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

const sampleClients: ClientDto[] = [
  {
    clientId: "aaaa-aaaa",
    name: "dev-linux",
    connectionState: "Connected",
    latencyMs: 2.5,
    eventsPerSecond: 60,
  },
];

beforeEach(() => {
  jest.useFakeTimers();
});

afterEach(() => {
  useMasterStore.setState({
    clients: [],
    isLoadingClients: false,
    lastError: null,
  });
  mockInvoke.mockReset();
  jest.useRealTimers();
});

describe("useClients", () => {
  test("fetches clients on mount and updates the store", async () => {
    // Arrange
    mockInvoke.mockResolvedValue({
      success: true,
      data: sampleClients,
      error: null,
    });

    // Act
    renderHook(() => useClients());

    // Assert – wait for async fetch to populate the store
    await waitFor(() => {
      const { clients } = useMasterStore.getState();
      expect(clients).toEqual(sampleClients);
    });
  });

  test("sets isLoadingClients to false after fetch completes", async () => {
    // Arrange
    mockInvoke.mockResolvedValue({
      success: true,
      data: sampleClients,
      error: null,
    });

    // Act
    renderHook(() => useClients());

    // Assert – loading flag is cleared once fetch resolves
    await waitFor(() => {
      expect(useMasterStore.getState().isLoadingClients).toBe(false);
    });
  });

  test("stores error message in store when fetch fails", async () => {
    // Arrange
    mockInvoke.mockRejectedValue(new Error("connection refused"));

    // Act
    renderHook(() => useClients());

    // Assert – error is propagated to the store
    await waitFor(() => {
      expect(useMasterStore.getState().lastError).toBe("connection refused");
    });
  });

  test("polls for client updates after interval elapses", async () => {
    // Arrange – first call returns empty list, second returns sampleClients
    mockInvoke
      .mockResolvedValueOnce({ success: true, data: [], error: null })
      .mockResolvedValueOnce({
        success: true,
        data: sampleClients,
        error: null,
      });

    // Act
    renderHook(() => useClients());

    // Wait for the initial fetch to complete
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });

    // Advance past the 2-second polling interval
    act(() => {
      jest.advanceTimersByTime(2100);
    });

    // Assert – second fetch executed and store updated
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(2);
      expect(useMasterStore.getState().clients).toEqual(sampleClients);
    });
  });

  test("clears polling interval on unmount", async () => {
    // Arrange
    mockInvoke.mockResolvedValue({ success: true, data: [], error: null });

    const { unmount } = renderHook(() => useClients());

    await waitFor(() => expect(mockInvoke).toHaveBeenCalledTimes(1));

    // Act – unmount the hook (cleanup should cancel the interval)
    unmount();

    // Advance well past the polling interval – no further calls expected
    act(() => {
      jest.advanceTimersByTime(10000);
    });

    // Assert – invoke was only called once (the initial mount call)
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });
});
