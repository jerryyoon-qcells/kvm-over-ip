/**
 * Settings: form for editing client configuration (master address, client name).
 *
 * # Two-copy pattern (controlled form)
 *
 * The form uses React's *controlled component* pattern:
 * - `masterAddress` and `clientName` are local `useState` variables that
 *   mirror the current input values.
 * - The Zustand store holds the last successfully saved values.
 *
 * When the user types in a field, only the local state changes — the store is
 * not updated until the form is submitted and the backend confirms success.
 * This prevents partially-typed values from leaking into the rest of the UI.
 *
 * When the store updates (e.g., on the initial load triggered by `App`), a
 * `useEffect` syncs the form fields to reflect the loaded values.
 *
 * # Validation
 *
 * Client-side validation checks that `clientName` is not blank before sending
 * to the backend.  The backend also validates this (see `update_client_settings`
 * in Rust) — the client-side check just provides faster feedback.
 *
 * # `noValidate`
 *
 * The `<form noValidate>` attribute disables the browser's built-in HTML5
 * validation popups (which have inconsistent styling across browsers).  We
 * render our own error messages via `localError` instead.
 *
 * # Dual error display
 *
 * Errors are stored in two places:
 * - `localError` — shown inline in the Settings panel (e.g., "Client name
 *   must not be empty").  Scoped to this component.
 * - `setLastError` — written to the global Zustand store, shown in
 *   `StatusDisplay` as a general error banner.  Clears automatically on
 *   the next successful operation.
 *
 * Both are set on failure so that the error is visible regardless of which
 * part of the UI the user is looking at.
 *
 * # `isSaving` state
 *
 * While the backend call is in flight, `isSaving` is `true`.  Both inputs and
 * the submit button are disabled during this time to prevent double-submission.
 */

import React, { useState, useEffect } from "react";
import { useClientStore } from "../store";
import { updateClientSettings } from "../api";

/** Props for Settings. */
interface SettingsProps {
  /** Additional CSS class names to apply to the outer container. */
  className?: string;
}

/**
 * Settings form.  Reads initial values from the Zustand store and writes
 * updates back through the Tauri IPC on submit.
 */
export const Settings: React.FC<SettingsProps> = ({ className = "" }) => {
  // Read store state — `storedSettings` is the last confirmed saved state
  const storedSettings = useClientStore((s) => s.settings);
  const setSettings = useClientStore((s) => s.setSettings);
  const setLastError = useClientStore((s) => s.setLastError);

  // Local form state — independent of the store until the user saves
  const [masterAddress, setMasterAddress] = useState(storedSettings.masterAddress);
  const [clientName, setClientName] = useState(storedSettings.clientName);
  const [isSaving, setIsSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);

  // Sync form fields when the store updates (e.g., on the initial data load
  // triggered by App's useEffect).  Without this sync, the form would show
  // empty strings even after the backend returned the saved settings.
  useEffect(() => {
    setMasterAddress(storedSettings.masterAddress);
    setClientName(storedSettings.clientName);
  }, [storedSettings]);

  /**
   * Handles form submission.
   *
   * 1. Prevents the default browser form submission (which would reload the page).
   * 2. Runs client-side validation.
   * 3. Sends the settings to the backend via `updateClientSettings`.
   * 4. On success, updates the store and shows a confirmation message.
   * 5. On failure, sets both local and global error messages.
   */
  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    // Prevent the browser from navigating away on form submit
    e.preventDefault();
    // Clear previous messages so the user sees fresh feedback
    setLocalError(null);
    setSaveSuccess(false);

    // Client-side validation: client name is required
    if (clientName.trim().length === 0) {
      setLocalError("Client name must not be empty.");
      return; // do not call the backend with invalid data
    }

    setIsSaving(true);
    try {
      // Send new settings to the Rust backend via Tauri IPC
      await updateClientSettings({ masterAddress, clientName });
      // Only update the store after the backend confirms success
      setSettings({ masterAddress, clientName });
      // Clear any previous global error
      setLastError(null);
      // Show success confirmation
      setSaveSuccess(true);
    } catch (err) {
      // Capture the error as a string regardless of its actual type
      const msg = err instanceof Error ? err.message : String(err);
      // Show the error inline in the settings panel
      setLocalError(msg);
      // Also surface it in the global status bar
      setLastError(msg);
    } finally {
      // Always re-enable the form after the operation completes (success or failure)
      setIsSaving(false);
    }
  };

  return (
    <div className={`settings ${className}`}>
      <h2 className="settings__title">Settings</h2>

      {/* Inline error message — shown when validation or save fails */}
      {localError !== null && (
        <div className="settings__error" role="alert" data-testid="settings-error">
          {localError}
        </div>
      )}

      {/* Success confirmation — shown briefly after a successful save */}
      {saveSuccess && (
        <div className="settings__success" role="status" data-testid="settings-success">
          Settings saved.
        </div>
      )}

      {/*
        `noValidate` disables browser HTML5 validation popups so we can render
        our own styled error messages via `localError` instead.
      */}
      <form onSubmit={(e) => void handleSubmit(e)} noValidate>
        {/* Master address field */}
        <div className="settings__field">
          <label htmlFor="master-address" className="settings__label">
            Master Address
          </label>
          <input
            id="master-address"
            type="text"
            className="settings__input"
            placeholder="192.168.1.10 (leave empty for auto-discover)"
            value={masterAddress}
            onChange={(e) => setMasterAddress(e.target.value)}
            data-testid="input-master-address"
            disabled={isSaving} // prevent edits while save is in flight
          />
        </div>

        {/* Client name field — required */}
        <div className="settings__field">
          <label htmlFor="client-name" className="settings__label">
            Client Name
          </label>
          <input
            id="client-name"
            type="text"
            className="settings__input"
            placeholder="My laptop"
            value={clientName}
            onChange={(e) => setClientName(e.target.value)}
            data-testid="input-client-name"
            disabled={isSaving}
            aria-required="true" // announces to screen readers that this field is required
          />
        </div>

        <div className="settings__actions">
          <button
            type="submit"
            className="settings__btn settings__btn--save"
            disabled={isSaving}
            data-testid="btn-save-settings"
          >
            {/* Button label changes to "Saving…" during the backend call */}
            {isSaving ? "Saving…" : "Save Settings"}
          </button>
        </div>
      </form>
    </div>
  );
};
