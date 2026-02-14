/**
 * LayoutEditor: visual drag-and-drop editor for arranging client screens
 * relative to the master monitor.
 *
 * Renders a scaled virtual screen canvas.  The master screen is fixed at
 * position (0, 0) and cannot be moved.  Each client screen is rendered as a
 * draggable tile.  Dragging updates the x/y offsets in local state; clicking
 * "Apply Layout" persists the changes to the backend via `useLayout.saveLayout`.
 *
 * # Two-copy pattern (local state vs store)
 *
 * The layout has two copies:
 * 1. **Zustand store** (`storeLayout`) — the last layout confirmed by the backend.
 * 2. **Local state** (`localLayout`) — the in-progress edits the user is making.
 *
 * Changes made by dragging only update `localLayout`.  Clicking "Apply" calls
 * `saveLayout(localLayout)` which writes to the backend and, on success, updates
 * the store.  "Reset" discards local changes by overwriting `localLayout` from
 * `storeLayout`.
 *
 * This pattern prevents accidental writes to the backend while the user is
 * still dragging tiles.
 *
 * # SCALE factor
 *
 * Virtual screen coordinates are in pixels (e.g., 1920×1080).  The canvas
 * would be enormous at 1:1 scale.  `SCALE = 0.1` renders every 10 virtual
 * pixels as 1 CSS pixel, making a 1920×1080 master screen appear as 192×108 px
 * on screen.
 *
 * Dragging converts the CSS pixel delta back to virtual pixels by dividing by
 * SCALE: `delta_virtual = delta_css / SCALE`.
 *
 * # Drag implementation
 *
 * Drag events are handled with raw DOM `mousemove` / `mouseup` listeners on
 * `window` (not on the tile itself).  Attaching to `window` means the drag
 * continues even if the cursor moves outside the tile or the canvas, which
 * feels more natural to the user.
 *
 * `dragStartRef` stores the cursor's last known position.  Each `mousemove`
 * computes the delta from the previous position (incremental deltas), which
 * avoids needing to remember the cursor's position at the start of the drag.
 */

import React, { useState, useCallback, useEffect, useRef } from "react";
import { useMasterStore } from "../store";
import { useLayout } from "../hooks/useLayout";
import type { ClientLayoutDto } from "../types";

/**
 * Scale factor: virtual pixels to CSS pixels for the canvas preview.
 * 0.1 means 10 virtual pixels = 1 CSS pixel.
 */
const SCALE = 0.1;

/**
 * Master screen dimensions.
 *
 * In a full implementation these would be fetched from the backend via a
 * dedicated Tauri command.  Using constants here is a simplification for the
 * initial release.
 */
const MASTER_WIDTH = 1920;
const MASTER_HEIGHT = 1080;

/** Props for a single ScreenTile. */
interface ScreenTileProps {
  /** Display label shown inside the tile (e.g., "Master" or the client name). */
  label: string;
  /** X offset in virtual screen pixels. */
  x: number;
  /** Y offset in virtual screen pixels. */
  y: number;
  /** Width in virtual screen pixels. */
  width: number;
  /** Height in virtual screen pixels. */
  height: number;
  /** `true` for the master tile, which is not draggable. */
  isMaster?: boolean;
  /**
   * Called on each mouse move during a drag with the incremental delta in
   * virtual screen pixels (not CSS pixels).
   */
  onDrag?: (dx: number, dy: number) => void;
}

/**
 * Renders a single screen region on the layout canvas.
 *
 * Positioned absolutely within the canvas container using CSS `left`/`top`
 * derived from the (x, y) props multiplied by SCALE.
 *
 * The master tile has `cursor: default` and ignores mousedown events.
 * Client tiles have `cursor: grab` and attach window-level mousemove/mouseup
 * listeners on mousedown to implement smooth dragging.
 */
const ScreenTile: React.FC<ScreenTileProps> = ({
  label,
  x,
  y,
  width,
  height,
  isMaster = false,
  onDrag,
}) => {
  /**
   * Stores the cursor's last known position during a drag.
   * Using `useRef` (not `useState`) because updating it must not trigger a
   * re-render — it is internal bookkeeping only.
   */
  const dragStartRef = useRef<{ mx: number; my: number } | null>(null);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      // Master tile is not draggable
      if (isMaster) return;
      e.preventDefault();

      // Record the starting cursor position in CSS pixels
      dragStartRef.current = { mx: e.clientX, my: e.clientY };

      const handleMouseMove = (me: MouseEvent) => {
        if (dragStartRef.current === null) return;
        // Compute incremental CSS pixel delta from the last known position
        const dx = (me.clientX - dragStartRef.current.mx) / SCALE;
        const dy = (me.clientY - dragStartRef.current.my) / SCALE;
        // Update the reference for the next mousemove event
        dragStartRef.current = { mx: me.clientX, my: me.clientY };
        // Notify the parent component with the virtual-pixel delta
        onDrag?.(dx, dy);
      };

      const handleMouseUp = () => {
        // Drag complete — clear the ref and remove the window listeners
        dragStartRef.current = null;
        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("mouseup", handleMouseUp);
      };

      // Listen on window so the drag continues if the cursor leaves the tile
      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
    },
    [isMaster, onDrag]
  );

  return (
    <div
      role="button"
      tabIndex={isMaster ? -1 : 0}
      aria-label={`Screen: ${label}`}
      className={`screen-tile ${isMaster ? "screen-tile--master" : "screen-tile--client"}`}
      style={{
        position: "absolute",
        left: x * SCALE,      // convert virtual pixels to CSS pixels
        top: y * SCALE,
        width: width * SCALE,
        height: height * SCALE,
        cursor: isMaster ? "default" : "grab",
        userSelect: "none",   // prevent text selection while dragging
      }}
      onMouseDown={handleMouseDown}
      data-testid={`screen-tile-${label}`}
    >
      <span className="screen-tile__label">{label}</span>
      <span className="screen-tile__dims">
        {width}×{height}
      </span>
    </div>
  );
};

/** Props for LayoutEditor. */
interface LayoutEditorProps {
  /** Additional CSS class names to apply to the outer container. */
  className?: string;
}

/**
 * Main layout editor component.
 *
 * - Fetches the persisted layout on mount via `useLayout()`.
 * - Maintains a local copy of the layout for in-progress edits.
 * - Renders a scaled canvas with draggable client tiles.
 * - "Apply Layout" sends the local edits to the backend.
 * - "Reset" discards local edits and restores the last saved layout.
 */
export const LayoutEditor: React.FC<LayoutEditorProps> = ({
  className = "",
}) => {
  const storeLayout = useMasterStore((s) => s.layout);
  const isLoadingLayout = useMasterStore((s) => s.isLoadingLayout);
  const lastError = useMasterStore((s) => s.lastError);

  const { saveLayout } = useLayout();

  // Local editable copy — updated by dragging, reset to store on "Reset"
  const [localLayout, setLocalLayout] = useState<ClientLayoutDto[]>(storeLayout);
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  // When the store loads new data (e.g., on first fetch), sync local state
  useEffect(() => {
    setLocalLayout(storeLayout);
  }, [storeLayout]);

  /**
   * Returns a drag handler for a specific client (identified by `clientId`).
   * Uses `useCallback` to produce a stable function reference — without it, a
   * new function would be created on every render, causing `ScreenTile` to
   * re-render even when the tile's position has not changed.
   *
   * The returned function adds the incremental (dx, dy) delta to the client's
   * current offsets and rounds to the nearest pixel.
   */
  const handleDrag = useCallback(
    (clientId: string) => (dx: number, dy: number) => {
      setLocalLayout((prev) =>
        prev.map((entry) =>
          entry.clientId === clientId
            ? {
                ...entry,
                xOffset: Math.round(entry.xOffset + dx),
                yOffset: Math.round(entry.yOffset + dy),
              }
            : entry
        )
      );
    },
    []
  );

  /** Sends `localLayout` to the backend.  Shows a saving indicator. */
  const handleApply = useCallback(async () => {
    setIsSaving(true);
    setSaveError(null);
    try {
      await saveLayout(localLayout);
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSaving(false);
    }
  }, [saveLayout, localLayout]);

  /** Discards local changes by copying the store layout back to local state. */
  const handleReset = useCallback(() => {
    setLocalLayout(storeLayout);
    setSaveError(null);
  }, [storeLayout]);

  if (isLoadingLayout) {
    return (
      <div className={`layout-editor layout-editor--loading ${className}`} role="status">
        Loading layout…
      </div>
    );
  }

  // Compute canvas dimensions to tightly fit all screens.
  // We add 40 CSS pixels of padding so tiles at the far edge are not clipped.
  const allScreens = [
    { x: 0, y: 0, w: MASTER_WIDTH, h: MASTER_HEIGHT },
    ...localLayout.map((c) => ({
      x: c.xOffset,
      y: c.yOffset,
      w: c.width,
      h: c.height,
    })),
  ];
  const canvasWidth =
    Math.max(...allScreens.map((s) => s.x + s.w)) * SCALE + 40;
  const canvasHeight =
    Math.max(...allScreens.map((s) => s.y + s.h)) * SCALE + 40;

  return (
    <div className={`layout-editor ${className}`}>
      <h2 className="layout-editor__title">Virtual Screen Layout</h2>

      {/* Error display: shows either a save error or the last global error */}
      {(saveError !== null || lastError !== null) && (
        <div
          className="layout-editor__error"
          role="alert"
          data-testid="layout-error"
        >
          {saveError ?? lastError}
        </div>
      )}

      {/* Canvas: positions all screen tiles using absolute positioning */}
      <div
        className="layout-editor__canvas"
        data-testid="layout-canvas"
        style={{
          position: "relative",
          width: canvasWidth,
          height: canvasHeight,
          background: "var(--colour-canvas-bg, #1a1a2e)",
          overflow: "hidden",
        }}
      >
        {/* Master screen — fixed at (0, 0), not draggable */}
        <ScreenTile
          label="Master"
          x={0}
          y={0}
          width={MASTER_WIDTH}
          height={MASTER_HEIGHT}
          isMaster
        />

        {/* Client screens — each is a draggable tile */}
        {localLayout.map((entry) => (
          <ScreenTile
            key={entry.clientId}
            label={entry.name}
            x={entry.xOffset}
            y={entry.yOffset}
            width={entry.width}
            height={entry.height}
            onDrag={handleDrag(entry.clientId)}
          />
        ))}
      </div>

      {/* Action buttons */}
      <div className="layout-editor__actions">
        <button
          type="button"
          className="layout-editor__btn layout-editor__btn--apply"
          onClick={() => void handleApply()}
          disabled={isSaving}
          data-testid="btn-apply-layout"
        >
          {isSaving ? "Saving…" : "Apply Layout"}
        </button>
        <button
          type="button"
          className="layout-editor__btn layout-editor__btn--reset"
          onClick={handleReset}
          disabled={isSaving}
          data-testid="btn-reset-layout"
        >
          Reset
        </button>
      </div>
    </div>
  );
};
