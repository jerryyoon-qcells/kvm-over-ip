/**
 * Tests for the `StatusDisplay` component.
 *
 * # What these tests verify
 *
 * - The three render states (loading, unavailable, normal) are all handled.
 * - Each status field (connection state, master address, client name, monitors)
 *   is rendered with the correct value.
 * - The "Discovering…" fallback appears when master address is empty.
 * - Error messages appear only when `lastError` is set.
 *
 * # How store state is injected
 *
 * `StatusDisplay` reads directly from the Zustand store — it has no props
 * for data.  Tests use `useClientStore.setState(...)` to inject specific store
 * state before rendering.  This is the recommended pattern for testing
 * components that use Zustand.
 *
 * # `makeStatus` helper
 *
 * The helper function builds a complete `ClientStatusDto` with defaults and
 * accepts a `Partial<ClientStatusDto>` for overrides.  This keeps individual
 * tests concise — a test only needs to specify the field it cares about:
 * ```ts
 * makeStatus({ masterAddress: "10.0.0.5" })
 * ```
 */

import React from "react";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom";
import { StatusDisplay } from "../components/StatusDisplay";
import { useClientStore } from "../store";
import type { ClientStatusDto } from "../types";

// ── Test isolation ─────────────────────────────────────────────────────────────

// Reset relevant store slices after each test
afterEach(() => {
  useClientStore.setState({ status: null, isLoading: false, lastError: null });
});

// ── Helpers ────────────────────────────────────────────────────────────────────

/**
 * Creates a `ClientStatusDto` with sensible defaults.
 * Pass overrides for only the fields your test cares about.
 */
const makeStatus = (overrides: Partial<ClientStatusDto> = {}): ClientStatusDto => ({
  connectionStatus: "Active",
  masterAddress: "192.168.1.10",
  clientName: "my-laptop",
  monitorCount: 2,
  ...overrides,
});

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("StatusDisplay", () => {
  test("shows loading message when loading and no status available", () => {
    // Arrange — simulate the first fetch being in-flight
    useClientStore.setState({ isLoading: true, status: null });

    // Act
    render(<StatusDisplay />);

    // Assert — loading indicator is visible
    expect(screen.getByRole("status")).toHaveTextContent(/loading/i);
  });

  test("shows unavailable message when status is null and not loading", () => {
    // Arrange — simulate a failed initial fetch (loading done, no data)
    useClientStore.setState({ status: null, isLoading: false });

    // Act
    render(<StatusDisplay />);

    // Assert
    expect(screen.getByRole("status")).toHaveTextContent(/unavailable/i);
  });

  test("renders connection status when status is present", () => {
    // Arrange
    useClientStore.setState({ status: makeStatus({ connectionStatus: "Active" }) });

    // Act
    render(<StatusDisplay />);

    // Assert
    expect(screen.getByTestId("status-connection-state")).toHaveTextContent("Active");
  });

  test("renders master address in the status list", () => {
    // Arrange
    useClientStore.setState({ status: makeStatus({ masterAddress: "10.0.0.5" }) });

    // Act
    render(<StatusDisplay />);

    // Assert
    expect(screen.getByTestId("status-master-address")).toHaveTextContent("10.0.0.5");
  });

  test("shows Discovering when master address is empty", () => {
    // Arrange — empty string means auto-discovery mode is active
    useClientStore.setState({ status: makeStatus({ masterAddress: "" }) });

    // Act
    render(<StatusDisplay />);

    // Assert — component shows "Discovering…" fallback
    expect(screen.getByTestId("status-master-address")).toHaveTextContent("Discovering…");
  });

  test("renders client name in the status list", () => {
    // Arrange
    useClientStore.setState({ status: makeStatus({ clientName: "gaming-rig" }) });

    // Act
    render(<StatusDisplay />);

    // Assert
    expect(screen.getByTestId("status-client-name")).toHaveTextContent("gaming-rig");
  });

  test("renders monitor count in the status list", () => {
    // Arrange
    useClientStore.setState({ status: makeStatus({ monitorCount: 3 }) });

    // Act
    render(<StatusDisplay />);

    // Assert
    expect(screen.getByTestId("status-monitor-count")).toHaveTextContent("3");
  });

  test("does not render error when lastError is null", () => {
    // Arrange
    useClientStore.setState({ status: makeStatus(), lastError: null });

    // Act
    render(<StatusDisplay />);

    // Assert — the error element must not be in the document at all
    expect(screen.queryByTestId("status-error")).not.toBeInTheDocument();
  });

  test("renders error message when lastError is set", () => {
    // Arrange
    useClientStore.setState({ status: makeStatus(), lastError: "connection timeout" });

    // Act
    render(<StatusDisplay />);

    // Assert
    expect(screen.getByTestId("status-error")).toHaveTextContent("connection timeout");
  });
});
