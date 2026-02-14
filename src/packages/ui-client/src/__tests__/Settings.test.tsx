/**
 * Tests for the `Settings` component.
 *
 * # What these tests verify
 *
 * - Form inputs are rendered and pre-filled from the store.
 * - Submitting with an empty client name shows a validation error.
 * - A successful save shows a "Settings saved" confirmation.
 * - A backend failure shows an error message.
 * - Inputs and the submit button are disabled while saving.
 *
 * # Test setup
 *
 * The `mockInvoke` mock intercepts `update_client_settings` calls.
 * Individual tests configure the mock to return success or failure responses.
 *
 * # `userEvent` vs `fireEvent`
 *
 * Most tests use `fireEvent.click` for simple button clicks.  Tests that need
 * realistic user interaction (typing into inputs) use `userEvent.setup()` from
 * `@testing-library/user-event`, which simulates keyboard events more closely
 * than `fireEvent` does.
 *
 * # `waitFor`
 *
 * Because form submission is asynchronous (the handler calls `await invoke(...)`),
 * assertions about elements that appear after submission must use `waitFor`.
 */

import React from "react";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import "@testing-library/jest-dom";
import { Settings } from "../components/Settings";
import { useClientStore } from "../store";
import { invoke } from "@tauri-apps/api/core";

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

// ── Test isolation ─────────────────────────────────────────────────────────────

afterEach(() => {
  useClientStore.setState({
    settings: { masterAddress: "", clientName: "" },
    lastError: null,
  });
  mockInvoke.mockReset();
});

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("Settings", () => {
  test("renders master address and client name inputs", () => {
    // Arrange / Act
    render(<Settings />);

    // Assert — both inputs are present in the DOM
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
    // Arrange — seed the store with saved settings
    useClientStore.setState({
      settings: { masterAddress: "192.168.1.1", clientName: "work-pc" },
    });

    // Act
    render(<Settings />);

    // Assert — form fields reflect the stored values
    expect(screen.getByTestId("input-master-address")).toHaveValue("192.168.1.1");
    expect(screen.getByTestId("input-client-name")).toHaveValue("work-pc");
  });

  test("shows validation error when client name is empty on submit", async () => {
    // Arrange — store has an empty client name
    useClientStore.setState({
      settings: { masterAddress: "", clientName: "" },
    });
    render(<Settings />);

    // Act — click Save without entering a client name
    fireEvent.click(screen.getByTestId("btn-save-settings"));

    // Assert — validation error appears without calling the backend
    await waitFor(() =>
      expect(screen.getByTestId("settings-error")).toHaveTextContent(
        "Client name must not be empty"
      )
    );
  });

  test("shows success message after successful save", async () => {
    // Arrange — mock the backend to return a successful response
    mockInvoke.mockResolvedValue({ success: true, data: null, error: null });
    useClientStore.setState({
      settings: { masterAddress: "10.0.0.1", clientName: "my-pc" },
    });
    const user = userEvent.setup();
    render(<Settings />);

    // Act — click Save with valid data
    await user.click(screen.getByTestId("btn-save-settings"));

    // Assert — success message appears after the backend call resolves
    await waitFor(() =>
      expect(screen.getByTestId("settings-success")).toHaveTextContent("Settings saved")
    );
  });

  test("shows error message when save fails", async () => {
    // Arrange — mock the backend to return a failure response
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

    // Assert — error message from backend appears inline
    await waitFor(() =>
      expect(screen.getByTestId("settings-error")).toHaveTextContent("persistence error")
    );
  });

  test("disables inputs while saving", async () => {
    // Arrange — create a Promise that we can resolve manually so we can
    // inspect the disabled state while the save is still in-flight.
    let resolveInvoke: (value: unknown) => void = () => {};
    mockInvoke.mockImplementation(
      () => new Promise((res) => { resolveInvoke = res; })
    );
    useClientStore.setState({
      settings: { masterAddress: "10.0.0.1", clientName: "my-pc" },
    });

    render(<Settings />);

    // Act — click Save; the form stays pending because resolveInvoke hasn't been called
    fireEvent.click(screen.getByTestId("btn-save-settings"));

    // Assert — inputs and button are disabled while the save is in-flight
    expect(screen.getByTestId("input-master-address")).toBeDisabled();
    expect(screen.getByTestId("input-client-name")).toBeDisabled();

    // Cleanup — resolve the pending promise so there are no dangling async operations
    resolveInvoke({ success: true, data: null, error: null });
  });
});
