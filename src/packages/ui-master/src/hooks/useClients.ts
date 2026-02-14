/**
 * useClients: hook for fetching and polling the connected client list.
 *
 * Polls the backend every 2 seconds to reflect live connection state changes
 * (new clients connecting, clients disconnecting, latency updates, etc.).
 *
 * # Why poll instead of using events? (for beginners)
 *
 * The KVM master backend does not (yet) push state change notifications to
 * the UI via Tauri events.  Polling is simpler to implement and is acceptable
 * for a status display that updates every 2 seconds — users will not notice a
 * 2-second delay in seeing a client's connection state change.
 *
 * A future enhancement could use `tauri::AppHandle::emit` from Rust and
 * `listen()` from TypeScript to receive push notifications, eliminating
 * unnecessary polls when nothing has changed.
 *
 * # Cancellation
 *
 * The `useEffect` cleanup function calls `clearInterval` to stop the polling
 * when the component that mounted this hook is unmounted.  Without this,
 * the interval would keep firing even after the component is gone, causing
 * memory leaks and "can't perform state update on unmounted component" errors.
 *
 * # `useCallback` for `fetchClients`
 *
 * `fetchClients` is wrapped in `useCallback` so that its reference stays
 * stable across renders.  If it were defined as a plain `async function`
 * inside the effect, a new function object would be created on every render,
 * which would cause the `useEffect` to restart the interval unnecessarily.
 */

import { useEffect, useCallback } from "react";
import { getClients } from "../api";
import { useMasterStore } from "../store";

/** Polling interval in milliseconds. 2000 ms = 2 seconds. */
const POLL_INTERVAL_MS = 2000;

/**
 * Subscribes to the client list.  Re-fetches on mount and every
 * `POLL_INTERVAL_MS` thereafter.
 *
 * Writes the fetched client list into the Zustand store via `setClients` so
 * that all components subscribed to `useMasterStore((s) => s.clients)`
 * automatically re-render with the latest data without prop drilling.
 *
 * Errors are captured and stored in `lastError` (displayed in StatusBar).
 * Errors do not crash the poll loop — the next interval will try again.
 */
export function useClients(): void {
  const setClients = useMasterStore((s) => s.setClients);
  const setLoadingClients = useMasterStore((s) => s.setLoadingClients);
  const setLastError = useMasterStore((s) => s.setLastError);

  /**
   * Fetches the client list from the backend and updates the store.
   *
   * Wrapped in `useCallback` so the function reference is stable and
   * `useEffect`'s dependency array does not trigger a restart on every render.
   */
  const fetchClients = useCallback(async () => {
    try {
      const clients = await getClients();
      setClients(clients);
      // Clear any previous error on success
      setLastError(null);
    } catch (err) {
      setLastError(err instanceof Error ? err.message : String(err));
    } finally {
      // Always clear the loading spinner after the first fetch completes
      setLoadingClients(false);
    }
  }, [setClients, setLoadingClients, setLastError]);

  useEffect(() => {
    // Show loading spinner on the first fetch
    setLoadingClients(true);
    // Fetch immediately (don't wait for the first interval)
    void fetchClients();

    // Then fetch again every POLL_INTERVAL_MS milliseconds
    const interval = setInterval(() => {
      void fetchClients();
    }, POLL_INTERVAL_MS);

    // Cleanup: clear the interval when the hook unmounts to prevent memory leaks
    return () => clearInterval(interval);
  }, [fetchClients, setLoadingClients]);
}
