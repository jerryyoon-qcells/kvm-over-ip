/**
 * useClients: hook for fetching and polling the connected client list.
 *
 * Polls the backend every 2 seconds to reflect live connection state changes.
 */

import { useEffect, useCallback } from "react";
import { getClients } from "../api";
import { useMasterStore } from "../store";

/** Polling interval in milliseconds. */
const POLL_INTERVAL_MS = 2000;

/**
 * Subscribes to the client list.  Re-fetches on mount and every
 * `POLL_INTERVAL_MS` thereafter.
 *
 * Exposes the client list via the Zustand store so all consumers
 * stay synchronised without prop drilling.
 */
export function useClients(): void {
  const setClients = useMasterStore((s) => s.setClients);
  const setLoadingClients = useMasterStore((s) => s.setLoadingClients);
  const setLastError = useMasterStore((s) => s.setLastError);

  const fetchClients = useCallback(async () => {
    try {
      const clients = await getClients();
      setClients(clients);
      setLastError(null);
    } catch (err) {
      setLastError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoadingClients(false);
    }
  }, [setClients, setLoadingClients, setLastError]);

  useEffect(() => {
    setLoadingClients(true);
    void fetchClients();

    const interval = setInterval(() => {
      void fetchClients();
    }, POLL_INTERVAL_MS);

    return () => clearInterval(interval);
  }, [fetchClients, setLoadingClients]);
}
