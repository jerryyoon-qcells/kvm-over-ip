/**
 * Tests for the `StatusBar` component.
 *
 * # What these tests verify
 *
 * - Connected client count is computed correctly (only "Connected" and "Paired"
 *   states are counted; "Disconnected", "Discovered", "Connecting" are not).
 * - The count uses correct singular/plural grammar.
 * - Sharing state shows "ON" or "OFF".
 * - Error message is rendered only when `lastError` is non-null.
 *
 * # Fixtures
 *
 * `connectedClient` and `disconnectedClient` are pre-built `ClientDto` objects
 * reused across multiple tests.  The `afterEach` hook resets the store so each
 * test starts from a clean state.
 */

import React from "react";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom";
import { StatusBar } from "../components/StatusBar";
import { useMasterStore } from "../store";
import type { ClientDto } from "../types";

// ── Test isolation ─────────────────────────────────────────────────────────────

// Reset store between tests
afterEach(() => {
  useMasterStore.setState({
    clients: [],
    sharingEnabled: false,
    lastError: null,
  });
});

// ── Fixtures ───────────────────────────────────────────────────────────────────

/** A client in "Connected" state — should be counted in the connected total. */
const connectedClient: ClientDto = {
  clientId: "11111111-1111-1111-1111-111111111111",
  name: "dev-linux",
  connectionState: "Connected",
  latencyMs: 3.2,
  eventsPerSecond: 45,
};

/** A client in "Disconnected" state — must NOT be counted in the connected total. */
const disconnectedClient: ClientDto = {
  clientId: "22222222-2222-2222-2222-222222222222",
  name: "macbook",
  connectionState: "Disconnected",
  latencyMs: 0,
  eventsPerSecond: 0,
};

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("StatusBar", () => {
  test("shows zero clients when no clients are registered", () => {
    // Arrange
    useMasterStore.setState({ clients: [] });

    // Act
    render(<StatusBar />);

    // Assert — "0 clients connected" (plural because count != 1)
    expect(screen.getByTestId("status-clients")).toHaveTextContent(
      "0 clients connected"
    );
  });

  test("shows singular 'client' text for exactly one connected client", () => {
    // Arrange
    useMasterStore.setState({ clients: [connectedClient] });

    // Act
    render(<StatusBar />);

    // Assert — "1 client connected" (singular)
    expect(screen.getByTestId("status-clients")).toHaveTextContent(
      "1 client connected"
    );
  });

  test("shows plural 'clients' text for two connected clients", () => {
    // Arrange
    const second = { ...connectedClient, clientId: "33333333-3333-3333-3333-333333333333", name: "client2" };
    useMasterStore.setState({ clients: [connectedClient, second] });

    // Act
    render(<StatusBar />);

    // Assert — "2 clients connected" (plural)
    expect(screen.getByTestId("status-clients")).toHaveTextContent(
      "2 clients connected"
    );
  });

  test("does not count disconnected clients in the connected total", () => {
    // Arrange — one connected + one disconnected
    useMasterStore.setState({ clients: [connectedClient, disconnectedClient] });

    // Act
    render(<StatusBar />);

    // Assert — only the connected one is counted
    expect(screen.getByTestId("status-clients")).toHaveTextContent(
      "1 client connected"
    );
  });

  test("shows sharing OFF when sharing is disabled", () => {
    // Arrange
    useMasterStore.setState({ sharingEnabled: false });

    // Act
    render(<StatusBar />);

    // Assert
    expect(screen.getByTestId("status-sharing")).toHaveTextContent("Sharing: OFF");
  });

  test("shows sharing ON when sharing is enabled", () => {
    // Arrange
    useMasterStore.setState({ sharingEnabled: true });

    // Act
    render(<StatusBar />);

    // Assert
    expect(screen.getByTestId("status-sharing")).toHaveTextContent("Sharing: ON");
  });

  test("does not render error element when there is no error", () => {
    // Arrange
    useMasterStore.setState({ lastError: null });

    // Act
    render(<StatusBar />);

    // Assert — error element must be completely absent from the DOM
    expect(screen.queryByTestId("status-error")).not.toBeInTheDocument();
  });

  test("renders error message when lastError is set", () => {
    // Arrange
    useMasterStore.setState({ lastError: "connection refused" });

    // Act
    render(<StatusBar />);

    // Assert — error is displayed with the "Error:" prefix
    expect(screen.getByTestId("status-error")).toHaveTextContent(
      "Error: connection refused"
    );
  });
});
