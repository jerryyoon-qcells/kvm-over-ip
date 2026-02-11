/**
 * useLayout: hook for fetching and persisting the virtual screen layout.
 */

import { useEffect, useCallback } from "react";
import { getLayout, updateLayout } from "../api";
import { useMasterStore } from "../store";
import type { ClientLayoutDto } from "../types";

/**
 * Loads the current layout on mount and exposes a `saveLayout` action.
 *
 * Returns `saveLayout` so components can submit layout changes without
 * knowing the underlying Tauri IPC mechanism.
 */
export function useLayout(): { saveLayout: (layout: ClientLayoutDto[]) => Promise<void> } {
  const setLayout = useMasterStore((s) => s.setLayout);
  const setLoadingLayout = useMasterStore((s) => s.setLoadingLayout);
  const setLastError = useMasterStore((s) => s.setLastError);

  useEffect(() => {
    setLoadingLayout(true);
    getLayout()
      .then((layout) => {
        setLayout(layout);
        setLastError(null);
      })
      .catch((err: unknown) => {
        setLastError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => {
        setLoadingLayout(false);
      });
  }, [setLayout, setLoadingLayout, setLastError]);

  const saveLayout = useCallback(
    async (layout: ClientLayoutDto[]) => {
      try {
        await updateLayout(layout);
        setLayout(layout);
        setLastError(null);
      } catch (err) {
        setLastError(err instanceof Error ? err.message : String(err));
        throw err;
      }
    },
    [setLayout, setLastError]
  );

  return { saveLayout };
}
