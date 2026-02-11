import React from "react";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom";
import { StatusDisplay } from "../components/StatusDisplay";
import { useClientStore } from "../store";
import type { ClientStatusDto } from "../types";

afterEach(() => {
  useClientStore.setState({ status: null, isLoading: false, lastError: null });
});

const makeStatus = (overrides: Partial<ClientStatusDto> = {}): ClientStatusDto => ({
  connectionStatus: "Active",
  masterAddress: "192.168.1.10",
  clientName: "my-laptop",
  monitorCount: 2,
  ...overrides,
});

describe("StatusDisplay", () => {
  test("shows loading message when loading and no status available", () => {
    // Arrange
    useClientStore.setState({ isLoading: true, status: null });

    // Act
    render(<StatusDisplay />);

    // Assert
    expect(screen.getByRole("status")).toHaveTextContent(/loading/i);
  });

  test("shows unavailable message when status is null and not loading", () => {
    // Arrange
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
    // Arrange
    useClientStore.setState({ status: makeStatus({ masterAddress: "" }) });

    // Act
    render(<StatusDisplay />);

    // Assert
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

    // Assert
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
