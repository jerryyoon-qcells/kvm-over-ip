/**
 * Settings: form for editing client configuration (master address, client name).
 */

import React, { useState, useEffect } from "react";
import { useClientStore } from "../store";
import { updateClientSettings } from "../api";

/** Props for Settings. */
interface SettingsProps {
  className?: string;
}

/**
 * Settings form.  Reads initial values from the Zustand store and writes
 * updates back through the Tauri IPC on submit.
 */
export const Settings: React.FC<SettingsProps> = ({ className = "" }) => {
  const storedSettings = useClientStore((s) => s.settings);
  const setSettings = useClientStore((s) => s.setSettings);
  const setLastError = useClientStore((s) => s.setLastError);

  const [masterAddress, setMasterAddress] = useState(storedSettings.masterAddress);
  const [clientName, setClientName] = useState(storedSettings.clientName);
  const [isSaving, setIsSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);

  // Sync form when store changes (e.g. on initial load).
  useEffect(() => {
    setMasterAddress(storedSettings.masterAddress);
    setClientName(storedSettings.clientName);
  }, [storedSettings]);

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setLocalError(null);
    setSaveSuccess(false);

    if (clientName.trim().length === 0) {
      setLocalError("Client name must not be empty.");
      return;
    }

    setIsSaving(true);
    try {
      await updateClientSettings({ masterAddress, clientName });
      setSettings({ masterAddress, clientName });
      setLastError(null);
      setSaveSuccess(true);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setLocalError(msg);
      setLastError(msg);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className={`settings ${className}`}>
      <h2 className="settings__title">Settings</h2>

      {localError !== null && (
        <div className="settings__error" role="alert" data-testid="settings-error">
          {localError}
        </div>
      )}

      {saveSuccess && (
        <div className="settings__success" role="status" data-testid="settings-success">
          Settings saved.
        </div>
      )}

      <form onSubmit={(e) => void handleSubmit(e)} noValidate>
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
            disabled={isSaving}
          />
        </div>

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
            aria-required="true"
          />
        </div>

        <div className="settings__actions">
          <button
            type="submit"
            className="settings__btn settings__btn--save"
            disabled={isSaving}
            data-testid="btn-save-settings"
          >
            {isSaving ? "Saving…" : "Save Settings"}
          </button>
        </div>
      </form>
    </div>
  );
};
