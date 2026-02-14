/**
 * useLayout: hook for fetching and persisting the virtual screen layout.
 *
 * Loads the current layout from the backend on mount and exposes a
 * `saveLayout` action for submitting user changes.
 *
 * # Separation of concerns
 *
 * This hook isolates the Tauri IPC calls from the `LayoutEditor` component.
 * `LayoutEditor` only calls `saveLayout(layout)` — it does not need to know
 * that the layout is stored via a Tauri command, or how errors are propagated
 * to the store.  This makes the component easier to test with a mock hook.
 *
 * # Local vs store layout
 *
 * The layout has two representations:
 * 1. **Zustand store** — the last layout successfully loaded from or saved to
 *    the backend.  This is the "ground truth" from the backend's perspective.
 * 2. **`LayoutEditor` local state** — a temporary copy the user is editing.
 *    Changes here do not affect the store until "Apply Layout" is clicked.
 *
 * This hook manages only the store side.  The `LayoutEditor` component manages
 * its own local state using `useState`.
 *
 * # Error handling
 *
 * Both the fetch and the save propagate errors to `lastError` in the store
 * (displayed in StatusBar).  `saveLayout` additionally re-throws the error
 * so that `LayoutEditor` can display an inline error message near the button.
 */

import { useEffect, useCallback } from "react";
import { getLayout, updateLayout } from "../api";
import { useMasterStore } from "../store";
import type { ClientLayoutDto } from "../types";

/**
 * Loads the current layout on mount and exposes a `saveLayout` action.
 *
 * @returns An object with a single `saveLayout` function that takes the new
 *   layout array, sends it to the backend, and updates the Zustand store on
 *   success.
 */
export function useLayout(): { saveLayout: (layout: ClientLayoutDto[]) => Promise<void> } {
  const setLayout = useMasterStore((s) => s.setLayout);
  const setLoadingLayout = useMasterStore((s) => s.setLoadingLayout);
  const setLastError = useMasterStore((s) => s.setLastError);

  // Fetch the layout once when the hook first mounts (component mounts)
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

  /**
   * Sends the updated layout to the backend and, on success, updates the
   * Zustand store so other components see the new layout immediately.
   *
   * Re-throws on failure so callers can display an inline error message.
   *
   * Wrapped in `useCallback` so the function reference is stable and does
   * not cause unnecessary re-renders in the `LayoutEditor`.
   *
   * @param layout - The complete new layout (all clients' positions).
   * @throws Propagates backend errors to the caller.
   */
  const saveLayout = useCallback(
    async (layout: ClientLayoutDto[]) => {
      try {
        await updateLayout(layout);
        // Update the store only after the backend confirms success
        setLayout(layout);
        setLastError(null);
      } catch (err) {
        setLastError(err instanceof Error ? err.message : String(err));
        throw err; // re-throw so LayoutEditor can show inline error
      }
    },
    [setLayout, setLastError]
  );

  return { saveLayout };
}
