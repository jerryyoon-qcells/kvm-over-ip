import React from "react";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import "@testing-library/jest-dom";
import { Settings } from "../components/Settings";
import { useClientStore } from "../store";
import { invoke } from "@tauri-apps/api/core";

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

afterEach(() => {
  useClientStore.setState({
    settings: { masterAddress: "", clientName: "" },
    lastError: null,
  });
  mockInvoke.mockReset();
});

describe("Settings", () => {
  test("renders master address and client name inputs", () => {
    // Arrange / Act
    render(<Settings />);

    // Assert
    expect(screen.getByTestId("input-master-address")).toBeInTheDocument();
    expect(screen.getByTestId("input-client-name")).toBeInTheDocument();
  });

  test("renders save button", () => {
    // Arrange / Act
    render(<Settings />);

    // Assert
    expect(screen.getByTestId("btn-save-settings")).toBeInTheDocument();
  });

  test("pre-fills inputs from the store", () => {
    // Arrange
    useClientStore.setState({
      settings: { masterAddress: "192.168.1.1", clientName: "work-pc" },
    });

    // Act
    render(<Settings />);

    // Assert
    expect(screen.getByTestId("input-master-address")).toHaveValue("192.168.1.1");
    expect(screen.getByTestId("input-client-name")).toHaveValue("work-pc");
  });

  test("shows validation error when client name is empty on submit", async () => {
    // Arrange
    useClientStore.setState({
      settings: { masterAddress: "", clientName: "" },
    });
    render(<Settings />);

    // Act
    fireEvent.click(screen.getByTestId("btn-save-settings"));

    // Assert
    await waitFor(() =>
      expect(screen.getByTestId("settings-error")).toHaveTextContent(
        "Client name must not be empty"
      )
    );
  });

  test("shows success message after successful save", async () => {
    // Arrange
    mockInvoke.mockResolvedValue({ success: true, data: null, error: null });
    useClientStore.setState({
      settings: { masterAddress: "10.0.0.1", clientName: "my-pc" },
    });
    const user = userEvent.setup();
    render(<Settings />);

    // Act
    await user.click(screen.getByTestId("btn-save-settings"));

    // Assert
    await waitFor(() =>
      expect(screen.getByTestId("settings-success")).toHaveTextContent("Settings saved")
    );
  });

  test("shows error message when save fails", async () => {
    // Arrange
    mockInvoke.mockResolvedValue({
      success: false,
      data: null,
      error: "persistence error",
    });
    useClientStore.setState({
      settings: { masterAddress: "10.0.0.1", clientName: "my-pc" },
    });
    const user = userEvent.setup();
    render(<Settings />);

    // Act
    await user.click(screen.getByTestId("btn-save-settings"));

    // Assert
    await waitFor(() =>
      expect(screen.getByTestId("settings-error")).toHaveTextContent("persistence error")
    );
  });

  test("disables inputs while saving", async () => {
    // Arrange
    let resolveInvoke: (value: unknown) => void = () => {};
    mockInvoke.mockImplementation(
      () => new Promise((res) => { resolveInvoke = res; })
    );
    useClientStore.setState({
      settings: { masterAddress: "10.0.0.1", clientName: "my-pc" },
    });

    render(<Settings />);

    // Act
    fireEvent.click(screen.getByTestId("btn-save-settings"));

    // Assert – inputs should be disabled while save is in flight
    expect(screen.getByTestId("input-master-address")).toBeDisabled();
    expect(screen.getByTestId("input-client-name")).toBeDisabled();

    // Cleanup
    resolveInvoke({ success: true, data: null, error: null });
  });
});
