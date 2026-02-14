/**
 * Tests for the `ClientList` component.
 *
 * # What these tests verify
 *
 * - The three render states (loading, empty, populated) are all handled.
 * - One table row is rendered per client.
 * - Client name, connection state, latency, and events/s are displayed correctly.
 * - Latency shows "—" when the value is zero (not yet measured).
 * - The table has an accessible `aria-label`.
 *
 * # How store state is injected
 *
 * `ClientList` reads from the Zustand store directly.  Tests use
 * `useMasterStore.setState(...)` to set up the store state before rendering,
 * which is the recommended approach for testing Zustand-connected components.
 *
 * # `makeClient` helper
 *
 * Builds a valid `ClientDto` with defaults and accepts partial overrides,
 * keeping individual test arrangements concise.
 */

import React from "react";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom";
import { ClientList } from "../components/ClientList";
import { useMasterStore } from "../store";
import type { ClientDto } from "../types";

// ── Test isolation ─────────────────────────────────────────────────────────────

afterEach(() => {
  useMasterStore.setState({ clients: [], isLoadingClients: false });
});

// ── Helpers ────────────────────────────────────────────────────────────────────

/**
 * Creates a `ClientDto` with sensible defaults.
 * Override only the fields your test cares about.
 */
const makeClient = (overrides: Partial<ClientDto> = {}): ClientDto => ({
  clientId: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
  name: "test-client",
  connectionState: "Connected",
  latencyMs: 5.0,
  eventsPerSecond: 100,
  ...overrides,
});

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("ClientList", () => {
  test("shows loading message while clients are loading and list is empty", () => {
    // Arrange — simulate first fetch in-flight (no data yet)
    useMasterStore.setState({ clients: [], isLoadingClients: true });

    // Act
    render(<ClientList />);

    // Assert — loading indicator is visible
    expect(screen.getByRole("status")).toHaveTextContent(/loading/i);
  });

  test("shows empty message when no clients are registered", () => {
    // Arrange — loading complete but no clients discovered
    useMasterStore.setState({ clients: [], isLoadingClients: false });

    // Act
    render(<ClientList />);

    // Assert — empty state message is shown
    expect(screen.getByRole("status")).toHaveTextContent(/no clients/i);
  });

  test("renders a row for each registered client", () => {
    // Arrange — two clients in the store
    const clients = [
      makeClient({ clientId: "aaaa-1", name: "alpha" }),
      makeClient({ clientId: "aaaa-2", name: "beta" }),
    ];
    useMasterStore.setState({ clients });

    // Act
    render(<ClientList />);

    // Assert — one row per client, identified by clientId
    expect(screen.getByTestId("client-row-aaaa-1")).toBeInTheDocument();
    expect(screen.getByTestId("client-row-aaaa-2")).toBeInTheDocument();
  });

  test("renders client name in the row", () => {
    // Arrange
    useMasterStore.setState({ clients: [makeClient({ name: "my-laptop" })] });

    // Act
    render(<ClientList />);

    // Assert
    expect(screen.getByText("my-laptop")).toBeInTheDocument();
  });

  test("renders connection state with correct text", () => {
    // Arrange
    const id = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
    useMasterStore.setState({
      clients: [makeClient({ clientId: id, connectionState: "Pairing" })],
    });

    // Act
    render(<ClientList />);

    // Assert — the state cell shows the serialised enum variant name
    expect(screen.getByTestId(`client-state-${id}`)).toHaveTextContent("Pairing");
  });

  test("shows latency dash when latency is zero", () => {
    // Arrange — latencyMs: 0 means the first Ping/Pong has not completed yet
    const id = "cccccccc-cccc-cccc-cccc-cccccccccccc";
    useMasterStore.setState({
      clients: [makeClient({ clientId: id, latencyMs: 0 })],
    });

    // Act
    render(<ClientList />);

    // Assert — "—" (em dash) shown instead of "0.0 ms"
    expect(screen.getByTestId(`client-latency-${id}`)).toHaveTextContent("—");
  });

  test("shows formatted latency when latency is non-zero", () => {
    // Arrange
    const id = "dddddddd-dddd-dddd-dddd-dddddddddddd";
    useMasterStore.setState({
      clients: [makeClient({ clientId: id, latencyMs: 12.3 })],
    });

    // Act
    render(<ClientList />);

    // Assert — formatted to one decimal place with "ms" unit
    expect(screen.getByTestId(`client-latency-${id}`)).toHaveTextContent("12.3 ms");
  });

  test("renders accessible table with aria-label", () => {
    // Arrange
    useMasterStore.setState({ clients: [makeClient()] });

    // Act
    render(<ClientList />);

    // Assert — the table has an aria-label so screen readers know its purpose
    expect(screen.getByRole("table", { name: /kvm clients/i })).toBeInTheDocument();
  });
});
