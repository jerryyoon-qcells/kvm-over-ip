/**
 * ClientList: displays all known KVM clients with their connection state,
 * latency, and events-per-second metrics.
 *
 * # Data flow
 *
 * This component reads data from the Zustand store; it does not fetch data
 * itself.  The `useClients()` hook (mounted in `App`) polls the backend and
 * writes to the store.  `ClientList` only needs to subscribe:
 *
 * ```text
 * useClients()                          ClientList
 * ──────────────────────────────────    ───────────────────────────
 * getClients() → setClients(data)  →   useMasterStore(s => s.clients)
 * ```
 *
 * # Accessibility
 *
 * The list uses a `<table>` with `scope="col"` header cells and `aria-label`
 * so screen readers can announce the table's purpose and navigate by column.
 *
 * # State colours
 *
 * Each connection state is mapped to a CSS custom property colour so that
 * the colour can be changed globally by updating the `:root` variables.
 * `inherit` is the fallback for unknown states.
 */

import React from "react";
import { useMasterStore } from "../store";
import type { ClientDto } from "../types";

/** Props for a single ClientRow. */
interface ClientRowProps {
  client: ClientDto;
}

/**
 * A single row in the client list table.
 *
 * Renders one client's name, connection state (colour-coded), latency, and
 * events-per-second.  Uses `data-testid` attributes for reliable test
 * targeting without coupling tests to CSS class names.
 *
 * Latency and events/s show "—" (em dash) when the value is zero to indicate
 * that the measurement is not yet available (e.g., the client just connected
 * and the first Ping/Pong cycle has not completed).
 */
const ClientRow: React.FC<ClientRowProps> = ({ client }) => {
  /**
   * Maps connection state strings to CSS colour variables.
   *
   * The values match the Rust `ConnectionState` enum variants serialised as
   * strings (via `Debug` formatting).  New states added to the Rust enum must
   * be added here to get a proper colour.
   */
  const stateColour: Record<string, string> = {
    Discovered: "var(--colour-info)",     // client found via UDP discovery
    Connecting: "var(--colour-warning)",  // TCP connect in progress
    Connected: "var(--colour-success)",   // control channel established
    Pairing: "var(--colour-warning)",     // PIN pairing in progress
    Paired: "var(--colour-success)",      // pairing complete, fully active
    Disconnected: "var(--colour-muted)",  // connection lost
  };

  // Fall back to `inherit` (default text colour) for unknown states
  const colour = stateColour[client.connectionState] ?? "inherit";

  return (
    <tr className="client-list__row" data-testid={`client-row-${client.clientId}`}>
      <td className="client-list__name">{client.name}</td>
      <td
        className="client-list__state"
        style={{ color: colour }}
        data-testid={`client-state-${client.clientId}`}
      >
        {client.connectionState}
      </td>
      <td className="client-list__latency" data-testid={`client-latency-${client.clientId}`}>
        {/* Show latency with one decimal place, or "—" if not yet measured */}
        {client.latencyMs > 0 ? `${client.latencyMs.toFixed(1)} ms` : "—"}
      </td>
      <td className="client-list__eps" data-testid={`client-eps-${client.clientId}`}>
        {/* Show events/s, or "—" if no events have been received yet */}
        {client.eventsPerSecond > 0 ? `${client.eventsPerSecond} ev/s` : "—"}
      </td>
    </tr>
  );
};

/** Props for ClientList. */
interface ClientListProps {
  /** Additional CSS class names to apply to the outer container. */
  className?: string;
}

/**
 * Renders the list of all known KVM clients.
 *
 * Reads from the Zustand store; call `useClients()` in a parent component
 * to keep the store up-to-date (the `App` component does this).
 *
 * Shows a "Loading…" message during the initial fetch, an "empty" message
 * if no clients have been discovered, and a table with one row per client
 * once data is available.
 */
export const ClientList: React.FC<ClientListProps> = ({ className = "" }) => {
  const clients = useMasterStore((s) => s.clients);
  const isLoading = useMasterStore((s) => s.isLoadingClients);

  // Show loading indicator only on the very first load (before any data)
  if (isLoading && clients.length === 0) {
    return (
      <div className={`client-list client-list--loading ${className}`} role="status">
        Loading clients…
      </div>
    );
  }

  // Show empty state message when no clients have been discovered yet
  if (clients.length === 0) {
    return (
      <div className={`client-list client-list--empty ${className}`} role="status">
        No clients discovered yet.
      </div>
    );
  }

  return (
    <div className={`client-list ${className}`}>
      <table className="client-list__table" aria-label="KVM clients">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">State</th>
            <th scope="col">Latency</th>
            <th scope="col">Events/s</th>
          </tr>
        </thead>
        <tbody>
          {clients.map((client) => (
            // Use clientId as the React key to ensure stable DOM identity
            // even if the client list order changes between polls
            <ClientRow key={client.clientId} client={client} />
          ))}
        </tbody>
      </table>
    </div>
  );
};
