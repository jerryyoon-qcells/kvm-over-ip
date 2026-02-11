/**
 * LayoutEditor: visual drag-and-drop editor for arranging client screens
 * relative to the master monitor.
 *
 * Renders a scaled virtual screen canvas.  The master screen is fixed at
 * position (0, 0).  Each client screen is rendered as a draggable tile.
 * Dragging updates the x/y offsets in local state; clicking "Apply" persists
 * the layout to the backend.
 */

import React, { useState, useCallback, useEffect, useRef } from "react";
import { useMasterStore } from "../store";
import { useLayout } from "../hooks/useLayout";
import type { ClientLayoutDto } from "../types";

/** Scale factor: virtual pixels to CSS pixels for the canvas preview. */
const SCALE = 0.1;

/** Master screen dimensions (could be read from backend in a full implementation). */
const MASTER_WIDTH = 1920;
const MASTER_HEIGHT = 1080;

/** Props for a single ScreenTile. */
interface ScreenTileProps {
  label: string;
  x: number;
  y: number;
  width: number;
  height: number;
  isMaster?: boolean;
  onDrag?: (dx: number, dy: number) => void;
}

/**
 * Renders a single screen region on the layout canvas.
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
  const dragStartRef = useRef<{ mx: number; my: number } | null>(null);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (isMaster) return;
      e.preventDefault();
      dragStartRef.current = { mx: e.clientX, my: e.clientY };

      const handleMouseMove = (me: MouseEvent) => {
        if (dragStartRef.current === null) return;
        const dx = (me.clientX - dragStartRef.current.mx) / SCALE;
        const dy = (me.clientY - dragStartRef.current.my) / SCALE;
        dragStartRef.current = { mx: me.clientX, my: me.clientY };
        onDrag?.(dx, dy);
      };

      const handleMouseUp = () => {
        dragStartRef.current = null;
        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("mouseup", handleMouseUp);
      };

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
        left: x * SCALE,
        top: y * SCALE,
        width: width * SCALE,
        height: height * SCALE,
        cursor: isMaster ? "default" : "grab",
        userSelect: "none",
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
  className?: string;
}

/**
 * Main layout editor component.
 *
 * - Fetches the persisted layout on mount via `useLayout()`.
 * - Allows dragging client tiles to adjust their position.
 * - Pressing "Apply Layout" sends the updated positions to the backend.
 */
export const LayoutEditor: React.FC<LayoutEditorProps> = ({
  className = "",
}) => {
  const storeLayout = useMasterStore((s) => s.layout);
  const isLoadingLayout = useMasterStore((s) => s.isLoadingLayout);
  const lastError = useMasterStore((s) => s.lastError);

  const { saveLayout } = useLayout();

  // Local editable copy of the layout – the store is only updated on Apply.
  const [localLayout, setLocalLayout] = useState<ClientLayoutDto[]>(storeLayout);
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  // Sync local state when store layout loads.
  useEffect(() => {
    setLocalLayout(storeLayout);
  }, [storeLayout]);

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

  // Canvas bounds: union of all screen regions.
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

      {(saveError !== null || lastError !== null) && (
        <div
          className="layout-editor__error"
          role="alert"
          data-testid="layout-error"
        >
          {saveError ?? lastError}
        </div>
      )}

      {/* Canvas */}
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
        {/* Master screen – fixed, not draggable */}
        <ScreenTile
          label="Master"
          x={0}
          y={0}
          width={MASTER_WIDTH}
          height={MASTER_HEIGHT}
          isMaster
        />

        {/* Client screens */}
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

      {/* Actions */}
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
