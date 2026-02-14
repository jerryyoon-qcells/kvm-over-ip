/**
 * Root application component for kvm-client UI.
 *
 * Renders a status panel (connection state, master address, monitors)
 * and a settings panel for configuration.
 *
 * # Component responsibilities
 *
 * `App` has two responsibilities:
 * 1. **Data loading** — on mount it fetches the current status and settings
 *    from the backend simultaneously (using `Promise.all` to minimise wait time).
 * 2. **Status polling** — a `setInterval` checks the connection status every
 *    2 seconds so the UI reflects live changes (e.g., when the master connects
 *    or disconnects).
 *
 * All fetched data is written into the Zustand store so child components
 * (`StatusDisplay`, `Settings`) can read it without prop drilling.
 *
 * # Why two separate `useEffect` calls?
 *
 * The initial load (status + settings) runs once on mount.  The polling loop
 * runs separately because:
 * - Settings do not need to be polled — they only change when the user submits
 *   the form.
 * - Separating them makes the dependency arrays cleaner and avoids restarting
 *   the interval whenever `setSettings` or `setLoading` changes identity.
 *
 * # `Promise.all` for concurrent initial fetch
 *
 * `Promise.all([getClientStatus(), getClientSettings()])` fires both requests
 * concurrently.  Without this, `await getClientStatus()` would finish before
 * `await getClientSettings()` even starts, doubling the load time.
 *
 * # Cleanup
 *
 * The second `useEffect` returns `() => clearInterval(interval)` so the
 * polling stops when the component unmounts (or when the effect re-runs due to
 * dependency changes).  Without cleanup the interval would keep firing even
 * after the component is gone, causing memory leaks.
 */

import React, { useEffect } from "react";
import { StatusDisplay } from "./components/StatusDisplay";
import { Settings } from "./components/Settings";
import { useClientStore } from "./store";
import { getClientStatus, getClientSettings } from "./api";

/** Polling interval for status refresh in milliseconds. */
const STATUS_POLL_MS = 2000;

/**
 * Root application component for the kvm-client Tauri window.
 *
 * Manages data fetching lifecycle and composes the two main UI sections:
 * connection status display and settings form.
 */
const App: React.FC = () => {
  // Read store action references (these are stable across renders)
  const setStatus = useClientStore((s) => s.setStatus);
  const setSettings = useClientStore((s) => s.setSettings);
  const setLoading = useClientStore((s) => s.setLoading);
  const setLastError = useClientStore((s) => s.setLastError);

  // ── Initial data load ──────────────────────────────────────────────────────
  // Runs once on mount.  Fetches both status and settings concurrently to
  // minimise the time before the UI shows real data.
  useEffect(() => {
    setLoading(true);

    // Promise.all fires both requests at the same time and waits for both
    // to finish before running the .then() handler.
    Promise.all([getClientStatus(), getClientSettings()])
      .then(([status, settings]) => {
        setStatus(status);
        setSettings(settings);
        setLastError(null);
      })
      .catch((err: unknown) => {
        // Store the error message so StatusDisplay can show it
        setLastError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => setLoading(false));
  }, [setStatus, setSettings, setLoading, setLastError]);

  // ── Periodic status polling ────────────────────────────────────────────────
  // Only polls the status (not settings) because settings only change when
  // the user explicitly submits the Settings form.
  useEffect(() => {
    const interval = setInterval(() => {
      getClientStatus()
        .then((s) => {
          setStatus(s);
          setLastError(null);
        })
        .catch((err: unknown) => {
          setLastError(err instanceof Error ? err.message : String(err));
        });
    }, STATUS_POLL_MS);

    // Return a cleanup function: clear the interval when this component
    // unmounts or before the effect re-runs.
    return () => clearInterval(interval);
  }, [setStatus, setLastError]);

  return (
    <div className="app" data-testid="client-app-root">
      <header className="app__header">
        <h1 className="app__title">KVM-Over-IP Client</h1>
      </header>

      <main className="app__main">
        {/* Connection status panel — shows live state from the backend */}
        <section aria-label="Connection status">
          <StatusDisplay />
        </section>

        {/* Settings form — allows configuring master address and client name */}
        <section aria-label="Client settings">
          <Settings />
        </section>
      </main>
    </div>
  );
};

export default App;
