/**
 * StatusBar: displays sharing state, error messages, and connected client count.
 */

import React from "react";
import { useMasterStore } from "../store";

/** Props for StatusBar. */
interface StatusBarProps {
  /** Additional CSS class names. */
  className?: string;
}

/**
 * Application status bar rendered at the bottom of the main layout.
 *
 * Shows:
 * - Number of connected clients
 * - Whether input sharing is active
 * - The most recent error (if any)
 */
export const StatusBar: React.FC<StatusBarProps> = ({ className = "" }) => {
  const clients = useMasterStore((s) => s.clients);
  const sharingEnabled = useMasterStore((s) => s.sharingEnabled);
  const lastError = useMasterStore((s) => s.lastError);

  const connectedCount = clients.filter(
    (c) => c.connectionState === "Connected" || c.connectionState === "Paired"
  ).length;

  return (
    <footer
      className={`status-bar ${className}`}
      role="status"
      aria-live="polite"
    >
      <span className="status-bar__clients" data-testid="status-clients">
        {connectedCount} client{connectedCount !== 1 ? "s" : ""} connected
      </span>

      <span
        className={`status-bar__sharing ${sharingEnabled ? "status-bar__sharing--active" : ""}`}
        data-testid="status-sharing"
      >
        Sharing: {sharingEnabled ? "ON" : "OFF"}
      </span>

      {lastError !== null && (
        <span className="status-bar__error" data-testid="status-error" role="alert">
          Error: {lastError}
        </span>
      )}
    </footer>
  );
};
