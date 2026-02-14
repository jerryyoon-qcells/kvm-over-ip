/**
 * Root application component for kvm-master UI.
 *
 * Renders a two-panel layout:
 * - Left panel (sidebar): ClientList showing all discovered/connected clients.
 * - Right panel (main content): LayoutEditor for arranging screen positions.
 * - Bottom: StatusBar with sharing state and error display.
 *
 * # Component responsibility
 *
 * `App` acts as the layout shell.  It:
 * 1. Mounts the `useClients()` polling hook once for the whole application so
 *    the client list is kept up-to-date without every child needing to start
 *    its own poll.
 * 2. Composes the three major UI sections into a semantic HTML structure
 *    (`header`, `main`, `aside`, `section`, `footer`).
 *
 * All application logic and state management lives in child components and
 * hooks; `App` itself only handles layout.
 *
 * # `useClients()` placement
 *
 * The hook is called here (not inside `ClientList`) so that its polling
 * interval starts when the app first renders and stays alive for the entire
 * session.  If it were called inside `ClientList`, unmounting `ClientList`
 * would stop the polling and the Zustand store would go stale.
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
  // This call starts a setInterval that re-fetches the client list every
  // 2 seconds and stores the result in useMasterStore.
  useClients();

  return (
    <div className="app" data-testid="app-root">
      <header className="app__header">
        <h1 className="app__title">KVM-Over-IP</h1>
      </header>

      <main className="app__main">
        {/* Left sidebar: list of discovered and connected KVM clients */}
        <aside className="app__sidebar">
          <section aria-label="Connected clients">
            <h2 className="section-heading">Clients</h2>
            <ClientList />
          </section>
        </aside>

        {/* Main content: drag-and-drop virtual screen layout editor */}
        <section className="app__content" aria-label="Screen layout editor">
          <LayoutEditor />
        </section>
      </main>

      {/* Bottom status bar: sharing state, client count, and error messages */}
      <StatusBar />
    </div>
  );
};

export default App;
