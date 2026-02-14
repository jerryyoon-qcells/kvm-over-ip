/**
 * StatusDisplay: shows the client's current connection status, master address,
 * client name, and monitor count.
 *
 * # Data source
 *
 * This component is purely presentational — it reads from the Zustand store
 * and renders whatever it finds.  The polling that keeps the data fresh is
 * done in the parent `App` component.  `StatusDisplay` does not fetch anything
 * itself.
 *
 * # Three render states
 *
 * 1. **Loading** — `isLoading` is `true` and `status` is `null` (the initial
 *    fetch has not returned yet).  Shows a "Loading…" spinner.
 * 2. **Unavailable** — `isLoading` is `false` but `status` is still `null`
 *    (the initial fetch failed).  Shows "Status unavailable."
 * 3. **Normal** — `status` is populated.  Renders the full status list.
 *
 * # Definition list `<dl>`
 *
 * The status fields are rendered using an HTML definition list (`<dl>`) with
 * `<dt>` (definition term) for field names and `<dd>` (definition description)
 * for values.  This is semantically correct for key/value data and is
 * announced clearly by screen readers.
 *
 * # Colour coding
 *
 * The connection status text is coloured using the `STATUS_COLOURS` map.
 * Colours are CSS custom properties (e.g., `var(--colour-success)`) so the
 * theme can be changed globally by updating the `:root` variables in CSS.
 * Hardcoded hex fallbacks (e.g., `#00c853`) ensure the colour works even if
 * the CSS variable is not defined.
 *
 * # `aria-live` and `role`
 *
 * The outer container has `role="region"` and `aria-label="Connection status"`
 * so screen reader users can navigate to it as a landmark.  Error messages
 * have `role="alert"` so they are announced immediately without the user
 * needing to navigate to them.
 */

import React from "react";
import { useClientStore } from "../store";
import type { ClientConnectionStatus } from "../types";

/**
 * Maps each `ClientConnectionStatus` string to a CSS colour expression.
 *
 * Uses a `Record<ClientConnectionStatus, string>` so TypeScript enforces
 * that every status variant has a colour — if a new variant is added to
 * the type, TypeScript will flag the missing entry here.
 */
const STATUS_COLOURS: Record<ClientConnectionStatus, string> = {
  Disconnected: "var(--colour-muted, #888)",          // grey — inactive
  Connecting: "var(--colour-warning, #f0a500)",        // amber — in progress
  Connected: "var(--colour-info, #0088ff)",            // blue — established
  Pairing: "var(--colour-warning, #f0a500)",           // amber — waiting for PIN
  Active: "var(--colour-success, #00c853)",            // green — fully operational
};

/** Props for StatusDisplay. */
interface StatusDisplayProps {
  /** Additional CSS class names to apply to the outer container. */
  className?: string;
}

/**
 * Displays a live snapshot of the client's connection state.
 *
 * Reads from the Zustand store; the parent `App` component polls the backend
 * every 2 seconds and writes updates to the store so this component stays
 * fresh without managing its own fetch lifecycle.
 */
export const StatusDisplay: React.FC<StatusDisplayProps> = ({
  className = "",
}) => {
  const status = useClientStore((s) => s.status);
  const isLoading = useClientStore((s) => s.isLoading);
  const lastError = useClientStore((s) => s.lastError);

  // Render loading indicator only during the initial fetch (before any data)
  if (isLoading && status === null) {
    return (
      <div className={`status-display status-display--loading ${className}`} role="status">
        Loading…
      </div>
    );
  }

  // Render unavailable message if the initial fetch failed and no data exists
  if (status === null) {
    return (
      <div className={`status-display status-display--unavailable ${className}`} role="status">
        Status unavailable.
      </div>
    );
  }

  // Look up the colour for the current connection state
  const colour = STATUS_COLOURS[status.connectionStatus];

  return (
    <div className={`status-display ${className}`} role="region" aria-label="Connection status">
      {/* Error banner — only shown when there is an active error */}
      {lastError !== null && (
        <div
          className="status-display__error"
          role="alert"
          data-testid="status-error"
        >
          {lastError}
        </div>
      )}

      {/*
        Definition list: each <dt> is a field name, each <dd> is its value.
        This structure is accessible and semantically correct for key/value data.
      */}
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
          {/*
            Show the master address if known; otherwise show "Discovering…"
            The empty string means auto-discovery is active (no static address).
          */}
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
