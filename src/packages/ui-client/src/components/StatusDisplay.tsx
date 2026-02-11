/**
 * StatusDisplay: shows the client's current connection status, master address,
 * and monitor count.
 */

import React from "react";
import { useClientStore } from "../store";
import type { ClientConnectionStatus } from "../types";

/** Colour mapping for connection status indicators. */
const STATUS_COLOURS: Record<ClientConnectionStatus, string> = {
  Disconnected: "var(--colour-muted, #888)",
  Connecting: "var(--colour-warning, #f0a500)",
  Connected: "var(--colour-info, #0088ff)",
  Pairing: "var(--colour-warning, #f0a500)",
  Active: "var(--colour-success, #00c853)",
};

/** Props for StatusDisplay. */
interface StatusDisplayProps {
  className?: string;
}

/**
 * Displays a live snapshot of the client's connection state.
 *
 * Reads from the Zustand store; call `getClientStatus()` from a parent
 * or polling hook to keep the data fresh.
 */
export const StatusDisplay: React.FC<StatusDisplayProps> = ({
  className = "",
}) => {
  const status = useClientStore((s) => s.status);
  const isLoading = useClientStore((s) => s.isLoading);
  const lastError = useClientStore((s) => s.lastError);

  if (isLoading && status === null) {
    return (
      <div className={`status-display status-display--loading ${className}`} role="status">
        Loading…
      </div>
    );
  }

  if (status === null) {
    return (
      <div className={`status-display status-display--unavailable ${className}`} role="status">
        Status unavailable.
      </div>
    );
  }

  const colour = STATUS_COLOURS[status.connectionStatus];

  return (
    <div className={`status-display ${className}`} role="region" aria-label="Connection status">
      {lastError !== null && (
        <div
          className="status-display__error"
          role="alert"
          data-testid="status-error"
        >
          {lastError}
        </div>
      )}

      <dl className="status-display__list">
        <dt>Status</dt>
        <dd
          style={{ color: colour }}
          data-testid="status-connection-state"
        >
          {status.connectionStatus}
        </dd>

        <dt>Master</dt>
        <dd data-testid="status-master-address">
          {status.masterAddress || "Discovering…"}
        </dd>

        <dt>Name</dt>
        <dd data-testid="status-client-name">{status.clientName}</dd>

        <dt>Monitors</dt>
        <dd data-testid="status-monitor-count">{status.monitorCount}</dd>
      </dl>
    </div>
  );
};
