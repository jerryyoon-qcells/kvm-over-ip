/**
 * Tests for the `useClients` hook.
 *
 * # What these tests verify
 *
 * - The hook fetches the client list on mount and writes it to the store.
 * - The `isLoadingClients` flag is cleared after the fetch completes.
 * - Backend errors are stored in `lastError` without crashing the hook.
 * - Polling fires again after the interval elapses and updates the store.
 * - Unmounting the hook clears the polling interval (no memory leaks).
 *
 * # Fake timers
 *
 * `jest.useFakeTimers()` replaces `setInterval` / `clearInterval` / `setTimeout`
 * with synchronous fake implementations that Jest controls.  This lets tests
 * advance "time" instantly using `jest.advanceTimersByTime(ms)` without waiting
 * for real wall-clock time to pass.
 *
 * `jest.useRealTimers()` in `afterEach` restores the real timer functions after
 * each test so other test files are not affected.
 *
 * # `act()` with timer advancement
 *
 * When advancing fake timers, React state updates triggered by the timer
 * callbacks must be wrapped in `act()`.  This tells React Testing Library to
 * flush all pending state updates synchronously before the assertions run.
 *
 * # `renderHook`
 *
 * `renderHook(() => useClients())` renders the hook in isolation without needing
 * a full React component.  The `{ unmount }` destructure lets tests simulate
 * component unmount to verify cleanup behaviour.
 */

import { renderHook, waitFor, act } from "@testing-library/react";
import "@testing-library/jest-dom";
import { useClients } from "../hooks/useClients";
import { useMasterStore } from "../store";
import { invoke } from "@tauri-apps/api/core";
import type { ClientDto } from "../types";

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

// ── Sample data ────────────────────────────────────────────────────────────────

const sampleClients: ClientDto[] = [
  {
    clientId: "aaaa-aaaa",
    name: "dev-linux",
    connectionState: "Connected",
    latencyMs: 2.5,
    eventsPerSecond: 60,
  },
];

// ── Setup / teardown ───────────────────────────────────────────────────────────

beforeEach(() => {
  // Use fake timers to control setInterval without real waiting
  jest.useFakeTimers();
});

afterEach(() => {
  // Reset store between tests to avoid state leakage
  useMasterStore.setState({
    clients: [],
    isLoadingClients: false,
    lastError: null,
  });
  mockInvoke.mockReset();
  // Restore real timers so other test files are not affected
  jest.useRealTimers();
});

// ── Tests ──────────────────────────────────────────────────────────────────────

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

    // Assert — wait for the async fetch to populate the store
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

    // Assert — loading flag is cleared once the fetch resolves
    await waitFor(() => {
      expect(useMasterStore.getState().isLoadingClients).toBe(false);
    });
  });

  test("stores error message in store when fetch fails", async () => {
    // Arrange — simulate a network error
    mockInvoke.mockRejectedValue(new Error("connection refused"));

    // Act
    renderHook(() => useClients());

    // Assert — the error is captured and stored (hook does not throw)
    await waitFor(() => {
      expect(useMasterStore.getState().lastError).toBe("connection refused");
    });
  });

  test("polls for client updates after interval elapses", async () => {
    // Arrange — first call returns empty list; second call returns clients
    mockInvoke
      .mockResolvedValueOnce({ success: true, data: [], error: null })
      .mockResolvedValueOnce({
        success: true,
        data: sampleClients,
        error: null,
      });

    // Act
    renderHook(() => useClients());

    // Wait for the initial (immediate) fetch to complete
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });

    // Advance the fake clock past the 2-second polling interval
    act(() => {
      jest.advanceTimersByTime(2100);
    });

    // Assert — the second fetch has been triggered and the store updated
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(2);
      expect(useMasterStore.getState().clients).toEqual(sampleClients);
    });
  });

  test("clears polling interval on unmount", async () => {
    // Arrange
    mockInvoke.mockResolvedValue({ success: true, data: [], error: null });

    const { unmount } = renderHook(() => useClients());

    // Wait for the initial fetch to complete
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledTimes(1));

    // Act — unmount the hook (cleanup function should call clearInterval)
    unmount();

    // Advance well past the polling interval
    act(() => {
      jest.advanceTimersByTime(10000);
    });

    // Assert — invoke was only called once (the initial mount call).
    // If clearInterval was NOT called, invoke would have been called multiple
    // additional times during the 10-second window.
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });
});
