import React from "react";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom";
import { ClientList } from "../components/ClientList";
import { useMasterStore } from "../store";
import type { ClientDto } from "../types";

afterEach(() => {
  useMasterStore.setState({ clients: [], isLoadingClients: false });
});

const makeClient = (overrides: Partial<ClientDto> = {}): ClientDto => ({
  clientId: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
  name: "test-client",
  connectionState: "Connected",
  latencyMs: 5.0,
  eventsPerSecond: 100,
  ...overrides,
});

describe("ClientList", () => {
  test("shows loading message while clients are loading and list is empty", () => {
    // Arrange
    useMasterStore.setState({ clients: [], isLoadingClients: true });

    // Act
    render(<ClientList />);

    // Assert
    expect(screen.getByRole("status")).toHaveTextContent(/loading/i);
  });

  test("shows empty message when no clients are registered", () => {
    // Arrange
    useMasterStore.setState({ clients: [], isLoadingClients: false });

    // Act
    render(<ClientList />);

    // Assert
    expect(screen.getByRole("status")).toHaveTextContent(/no clients/i);
  });

  test("renders a row for each registered client", () => {
    // Arrange
    const clients = [
      makeClient({ clientId: "aaaa-1", name: "alpha" }),
      makeClient({ clientId: "aaaa-2", name: "beta" }),
    ];
    useMasterStore.setState({ clients });

    // Act
    render(<ClientList />);

    // Assert
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

    // Assert
    expect(screen.getByTestId(`client-state-${id}`)).toHaveTextContent("Pairing");
  });

  test("shows latency dash when latency is zero", () => {
    // Arrange
    const id = "cccccccc-cccc-cccc-cccc-cccccccccccc";
    useMasterStore.setState({
      clients: [makeClient({ clientId: id, latencyMs: 0 })],
    });

    // Act
    render(<ClientList />);

    // Assert
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

    // Assert
    expect(screen.getByTestId(`client-latency-${id}`)).toHaveTextContent("12.3 ms");
  });

  test("renders accessible table with aria-label", () => {
    // Arrange
    useMasterStore.setState({ clients: [makeClient()] });

    // Act
    render(<ClientList />);

    // Assert
    expect(screen.getByRole("table", { name: /kvm clients/i })).toBeInTheDocument();
  });
});
