/**
 * Tests for the root App component.
 *
 * Verifies that App renders without crashing and mounts all key
 * child sections.  Child components (ClientList, LayoutEditor,
 * StatusBar) are exercised in their own test files; here we only
 * confirm structural presence.
 */

import React from "react";
import { render, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom";
import App from "../App";
import { useMasterStore } from "../store";
import { invoke } from "@tauri-apps/api/core";

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

beforeEach(() => {
  // Provide default successful responses for all IPC calls made on mount
  // (useClients polls get_clients; useLayout fetches get_layout).
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
  useMasterStore.setState({
    clients: [],
    layout: [],
    isLoadingClients: false,
    isLoadingLayout: false,
    lastError: null,
  });
  mockInvoke.mockReset();
});

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
    expect(
      screen.getByRole("complementary", { hidden: true })
    ).toBeInTheDocument();
  });

  test("renders the layout editor section", async () => {
    render(<App />);
    // The layout section is a <section> with aria-label
    expect(
      screen.getByRole("region", { name: /screen layout editor/i })
    ).toBeInTheDocument();
  });

  test("resolves loading state after successful IPC calls", async () => {
    render(<App />);

    // After mocked IPC resolves, the layout canvas should appear
    await waitFor(() =>
      expect(screen.getByTestId("layout-canvas")).toBeInTheDocument()
    );
  });
});
