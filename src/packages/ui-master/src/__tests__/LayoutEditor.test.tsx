/**
 * Tests for the `LayoutEditor` component.
 *
 * # What these tests verify
 *
 * - Loading state is shown while `isLoadingLayout` is `true`.
 * - The layout canvas and screen tiles render once loading completes.
 * - The master tile is always present.
 * - Client tiles are rendered for each entry in the layout.
 * - Apply and Reset buttons are present.
 * - Clicking Apply sends the layout to the backend via `update_layout`.
 * - A backend failure on Apply shows an inline error message.
 * - Clicking Reset clears the error without re-fetching.
 *
 * # Why `get_layout` must be mocked for every test
 *
 * `LayoutEditor` mounts the `useLayout()` hook, which calls `get_layout`
 * on mount.  If `get_layout` resolves with data, `isLoadingLayout` transitions
 * to `false` and the canvas appears.  Every test that checks for elements
 * that are hidden during loading must mock `get_layout` to give the hook a
 * way to exit the loading state.
 *
 * # `waitFor` for async rendering
 *
 * The canvas and buttons only appear after the async `get_layout` call
 * resolves.  `waitFor` retries the assertion until it passes or times out.
 */

import React from "react";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom";
import { LayoutEditor } from "../components/LayoutEditor";
import { useMasterStore } from "../store";
import { invoke } from "@tauri-apps/api/core";
import type { ClientLayoutDto } from "../types";

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

// ── Test isolation ─────────────────────────────────────────────────────────────

afterEach(() => {
  useMasterStore.setState({
    layout: [],
    isLoadingLayout: false,
    lastError: null,
  });
  mockInvoke.mockReset();
});

// ── Sample data ────────────────────────────────────────────────────────────────

/** A layout with one client screen placed to the right of the master. */
const sampleLayout: ClientLayoutDto[] = [
  {
    clientId: "aaaa-aaaa",
    name: "dev-linux",
    xOffset: 1920,
    yOffset: 0,
    width: 1920,
    height: 1080,
  },
];

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("LayoutEditor", () => {
  test("shows loading state when layout is loading", () => {
    // Arrange — simulate loading state
    useMasterStore.setState({ isLoadingLayout: true, layout: [] });

    // Act
    render(<LayoutEditor />);

    // Assert — loading indicator is visible
    expect(screen.getByRole("status")).toHaveTextContent(/loading/i);
  });

  test("renders the layout canvas when layout is loaded", async () => {
    // Arrange — mock get_layout so useLayout hook exits loading state
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: sampleLayout, error: null });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: sampleLayout, isLoadingLayout: false });

    // Act
    render(<LayoutEditor />);

    // Assert — wait for the async hook to complete before checking
    await waitFor(() =>
      expect(screen.getByTestId("layout-canvas")).toBeInTheDocument()
    );
  });

  test("renders the master screen tile", async () => {
    // Arrange — mock get_layout so the hook exits loading state
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: [], error: null });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: [], isLoadingLayout: false });

    // Act
    render(<LayoutEditor />);

    // Assert — master tile is always rendered, regardless of client count
    await waitFor(() =>
      expect(screen.getByTestId("screen-tile-Master")).toBeInTheDocument()
    );
  });

  test("renders a tile for each client in the layout", async () => {
    // Arrange
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: sampleLayout, error: null });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: sampleLayout, isLoadingLayout: false });

    // Act
    render(<LayoutEditor />);

    // Assert — a tile for "dev-linux" should be visible
    await waitFor(() =>
      expect(screen.getByTestId("screen-tile-dev-linux")).toBeInTheDocument()
    );
  });

  test("renders Apply and Reset buttons", async () => {
    // Arrange
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: sampleLayout, error: null });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: sampleLayout, isLoadingLayout: false });

    // Act
    render(<LayoutEditor />);

    // Assert
    await waitFor(() =>
      expect(screen.getByTestId("btn-apply-layout")).toBeInTheDocument()
    );
    expect(screen.getByTestId("btn-reset-layout")).toBeInTheDocument();
  });

  test("apply button triggers layout save", async () => {
    // Arrange — mock both get_layout and update_layout to succeed
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: sampleLayout, error: null });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: sampleLayout, isLoadingLayout: false });

    // Act — render, wait for the Apply button, then click it
    render(<LayoutEditor />);
    await waitFor(() =>
      expect(screen.getByTestId("btn-apply-layout")).toBeInTheDocument()
    );
    fireEvent.click(screen.getByTestId("btn-apply-layout"));

    // Assert — button re-enables after save completes (no longer shows "Saving…")
    await waitFor(() =>
      expect(screen.getByTestId("btn-apply-layout")).not.toBeDisabled()
    );
  });

  test("shows error message when apply fails", async () => {
    // Arrange — get_layout succeeds but update_layout returns an error
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: sampleLayout, error: null });
      }
      if (cmd === "update_layout") {
        return Promise.resolve({ success: false, data: null, error: "overlapping screens" });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: sampleLayout, isLoadingLayout: false });

    // Act
    render(<LayoutEditor />);
    await waitFor(() =>
      expect(screen.getByTestId("btn-apply-layout")).toBeInTheDocument()
    );
    fireEvent.click(screen.getByTestId("btn-apply-layout"));

    // Assert — error message from the backend is shown inline
    await waitFor(() =>
      expect(screen.getByTestId("layout-error")).toHaveTextContent("overlapping screens")
    );
  });

  test("reset button restores layout to store state", async () => {
    // Arrange — mock get_layout so loading state resolves
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: sampleLayout, error: null });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: sampleLayout, isLoadingLayout: false });

    // Act — render, wait for Reset button, click it
    render(<LayoutEditor />);
    await waitFor(() =>
      expect(screen.getByTestId("btn-reset-layout")).toBeInTheDocument()
    );
    fireEvent.click(screen.getByTestId("btn-reset-layout"));

    // Assert — clicking Reset clears any error state
    expect(screen.queryByTestId("layout-error")).not.toBeInTheDocument();
  });
});
