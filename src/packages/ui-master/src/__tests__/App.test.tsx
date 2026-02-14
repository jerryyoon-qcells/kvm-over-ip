/**
 * Tests for the root App component.
 *
 * # What these tests verify
 *
 * Confirms that App renders without crashing and mounts all key child
 * sections.  Child components (ClientList, LayoutEditor, StatusBar) are
 * exercised in their own test files; here we only confirm structural presence.
 *
 * # IPC mock setup
 *
 * App mounts two hooks on render:
 * - `useClients()` — immediately calls `get_clients` and then polls every 2s.
 * - `useLayout()` inside `LayoutEditor` — calls `get_layout` on mount.
 *
 * The `beforeEach` mock provides default successful responses for these two
 * commands so the components can exit their loading states and render their
 * normal content.
 *
 * # `waitFor` for async rendering
 *
 * Some assertions wait for async effects to complete.  For example,
 * `layout-canvas` only appears once `get_layout` resolves and the
 * `isLoadingLayout` flag clears.
 */

import React from "react";
import { render, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom";
import App from "../App";
import { useMasterStore } from "../store";
import { invoke } from "@tauri-apps/api/core";

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

// ── Setup / teardown ───────────────────────────────────────────────────────────

beforeEach(() => {
  // Provide default successful responses for all IPC calls made on mount.
  // `get_clients` is called by useClients(); `get_layout` is called by useLayout().
  mockInvoke.mockImplementation((cmd) => {
    if (cmd === "get_clients") {
      return Promise.resolve({ success: true, data: [], error: null });
    }
    if (cmd === "get_layout") {
      return Promise.resolve({ success: true, data: [], error: null });
    }
    return Promise.resolve({ success: true, data: null, error: null });
  });
});

afterEach(() => {
  // Reset store to avoid state leakage between tests
  useMasterStore.setState({
    clients: [],
    layout: [],
    isLoadingClients: false,
    isLoadingLayout: false,
    lastError: null,
  });
  mockInvoke.mockReset();
});

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("App", () => {
  test("renders the application root element", async () => {
    render(<App />);
    expect(screen.getByTestId("app-root")).toBeInTheDocument();
  });

  test("renders the application title heading", async () => {
    render(<App />);
    expect(
      screen.getByRole("heading", { name: /kvm-over-ip/i })
    ).toBeInTheDocument();
  });

  test("renders the clients sidebar section", async () => {
    render(<App />);
    // The <aside> element has role="complementary"
    expect(
      screen.getByRole("complementary", { hidden: true })
    ).toBeInTheDocument();
  });

  test("renders the layout editor section", async () => {
    render(<App />);
    // The layout <section> has aria-label="Screen layout editor"
    expect(
      screen.getByRole("region", { name: /screen layout editor/i })
    ).toBeInTheDocument();
  });

  test("resolves loading state after successful IPC calls", async () => {
    render(<App />);

    // The layout-canvas only appears once `get_layout` resolves and
    // isLoadingLayout is set back to false by the useLayout hook
    await waitFor(() =>
      expect(screen.getByTestId("layout-canvas")).toBeInTheDocument()
    );
  });
});
