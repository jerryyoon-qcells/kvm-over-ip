/**
 * StatusBar: displays sharing state, error messages, and connected client count.
 *
 * Rendered at the bottom of the main layout as a `<footer>` element.
 * Uses `aria-live="polite"` so screen readers announce status changes without
 * interrupting the user.
 *
 * # What each field shows
 *
 * - **Connected clients** — counts only clients in "Connected" or "Paired" state.
 *   Clients that are discovered but not yet paired, or disconnected, are not
 *   counted.
 * - **Sharing** — reflects `useMasterStore.sharingEnabled`.  When `true`, the
 *   master is forwarding keyboard and mouse events to the active client.
 * - **Error** — the most recent error from any hook or component.  Cleared
 *   by the hook that encountered the error on the next successful operation.
 *
 * # Role of `aria-live`
 *
 * `aria-live="polite"` tells assistive technologies (screen readers) to
 * announce any changes to this element's text content when the user is idle.
 * This allows visually impaired users to be informed of connection state
 * changes and errors without needing to navigate to the status bar manually.
 */

import React from "react";
import { useMasterStore } from "../store";

/** Props for StatusBar. */
interface StatusBarProps {
  /** Additional CSS class names to apply to the `<footer>` element. */
  className?: string;
}

/**
 * Application status bar rendered at the bottom of the main layout.
 *
 * Reads from the Zustand store (no props needed) and re-renders automatically
 * when the relevant store fields change.
 */
export const StatusBar: React.FC<StatusBarProps> = ({ className = "" }) => {
  const clients = useMasterStore((s) => s.clients);
  const sharingEnabled = useMasterStore((s) => s.sharingEnabled);
  const lastError = useMasterStore((s) => s.lastError);

  // Count only the clients that are fully connected and ready to receive input.
  // "Discovered" and "Connecting" clients are not counted.
  const connectedCount = clients.filter(
    (c) => c.connectionState === "Connected" || c.connectionState === "Paired"
  ).length;

  return (
    <footer
      className={`status-bar ${className}`}
      role="status"
      aria-live="polite"
    >
      {/* Client count — pluralises "client" correctly */}
      <span className="status-bar__clients" data-testid="status-clients">
        {connectedCount} client{connectedCount !== 1 ? "s" : ""} connected
      </span>

      {/* Sharing toggle state — highlighted when active */}
      <span
        className={`status-bar__sharing ${sharingEnabled ? "status-bar__sharing--active" : ""}`}
        data-testid="status-sharing"
      >
        Sharing: {sharingEnabled ? "ON" : "OFF"}
      </span>

      {/* Error message — only rendered when an error exists */}
      {lastError !== null && (
        <span className="status-bar__error" data-testid="status-error" role="alert">
          Error: {lastError}
        </span>
      )}
    </footer>
  );
};
