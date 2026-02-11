/**
 * Root application component for kvm-client UI.
 *
 * Renders a status panel (connection state, master address, monitors)
 * and a settings panel for configuration.
 */

import React, { useEffect } from "react";
import { StatusDisplay } from "./components/StatusDisplay";
import { Settings } from "./components/Settings";
import { useClientStore } from "./store";
import { getClientStatus, getClientSettings } from "./api";

/** Polling interval for status refresh in milliseconds. */
const STATUS_POLL_MS = 2000;

const App: React.FC = () => {
  const setStatus = useClientStore((s) => s.setStatus);
  const setSettings = useClientStore((s) => s.setSettings);
  const setLoading = useClientStore((s) => s.setLoading);
  const setLastError = useClientStore((s) => s.setLastError);

  // Initial data load.
  useEffect(() => {
    setLoading(true);

    Promise.all([getClientStatus(), getClientSettings()])
      .then(([status, settings]) => {
        setStatus(status);
        setSettings(settings);
        setLastError(null);
      })
      .catch((err: unknown) => {
        setLastError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => setLoading(false));
  }, [setStatus, setSettings, setLoading, setLastError]);

  // Periodic status polling.
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

    return () => clearInterval(interval);
  }, [setStatus, setLastError]);

  return (
    <div className="app" data-testid="client-app-root">
      <header className="app__header">
        <h1 className="app__title">KVM-Over-IP Client</h1>
      </header>

      <main className="app__main">
        <section aria-label="Connection status">
          <StatusDisplay />
        </section>

        <section aria-label="Client settings">
          <Settings />
        </section>
      </main>
    </div>
  );
};

export default App;
