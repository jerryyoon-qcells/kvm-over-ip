/**
 * ClientList: displays all known KVM clients with their connection state,
 * latency, and events-per-second metrics.
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
 */
const ClientRow: React.FC<ClientRowProps> = ({ client }) => {
  const stateColour: Record<string, string> = {
    Discovered: "var(--colour-info)",
    Connecting: "var(--colour-warning)",
    Connected: "var(--colour-success)",
    Pairing: "var(--colour-warning)",
    Paired: "var(--colour-success)",
    Disconnected: "var(--colour-muted)",
  };

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
        {client.latencyMs > 0 ? `${client.latencyMs.toFixed(1)} ms` : "—"}
      </td>
      <td className="client-list__eps" data-testid={`client-eps-${client.clientId}`}>
        {client.eventsPerSecond > 0 ? `${client.eventsPerSecond} ev/s` : "—"}
      </td>
    </tr>
  );
};

/** Props for ClientList. */
interface ClientListProps {
  className?: string;
}

/**
 * Renders the list of all known KVM clients.
 *
 * Reads from the Zustand store; call `useClients()` in a parent to keep
 * the store up-to-date.
 */
export const ClientList: React.FC<ClientListProps> = ({ className = "" }) => {
  const clients = useMasterStore((s) => s.clients);
  const isLoading = useMasterStore((s) => s.isLoadingClients);

  if (isLoading && clients.length === 0) {
    return (
      <div className={`client-list client-list--loading ${className}`} role="status">
        Loading clients…
      </div>
    );
  }

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
            <ClientRow key={client.clientId} client={client} />
          ))}
        </tbody>
      </table>
    </div>
  );
};
