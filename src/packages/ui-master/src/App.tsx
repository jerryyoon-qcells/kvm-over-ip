/**
 * Root application component for kvm-master UI.
 *
 * Renders a two-panel layout:
 * - Left panel: ClientList showing all discovered/connected clients.
 * - Right panel: LayoutEditor for arranging screen positions.
 * - Bottom: StatusBar with sharing state and error display.
 */

import React from "react";
import { ClientList } from "./components/ClientList";
import { LayoutEditor } from "./components/LayoutEditor";
import { StatusBar } from "./components/StatusBar";
import { useClients } from "./hooks/useClients";

/**
 * Root application component.
 *
 * Mounts the client polling loop via `useClients()` so that the client
 * list stays up-to-date across the entire application lifetime.
 */
const App: React.FC = () => {
  // Mount client polling loop; updates the Zustand store globally.
  useClients();

  return (
    <div className="app" data-testid="app-root">
      <header className="app__header">
        <h1 className="app__title">KVM-Over-IP</h1>
      </header>

      <main className="app__main">
        <aside className="app__sidebar">
          <section aria-label="Connected clients">
            <h2 className="section-heading">Clients</h2>
            <ClientList />
          </section>
        </aside>

        <section className="app__content" aria-label="Screen layout editor">
          <LayoutEditor />
        </section>
      </main>

      <StatusBar />
    </div>
  );
};

export default App;
