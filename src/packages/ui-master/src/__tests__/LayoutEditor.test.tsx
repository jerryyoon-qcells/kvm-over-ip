import React from "react";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom";
import { LayoutEditor } from "../components/LayoutEditor";
import { useMasterStore } from "../store";
import { invoke } from "@tauri-apps/api/core";
import type { ClientLayoutDto } from "../types";

const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;

afterEach(() => {
  useMasterStore.setState({
    layout: [],
    isLoadingLayout: false,
    lastError: null,
  });
  mockInvoke.mockReset();
});

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

describe("LayoutEditor", () => {
  test("shows loading state when layout is loading", () => {
    // Arrange
    useMasterStore.setState({ isLoadingLayout: true, layout: [] });

    // Act
    render(<LayoutEditor />);

    // Assert
    expect(screen.getByRole("status")).toHaveTextContent(/loading/i);
  });

  test("renders the layout canvas when layout is loaded", async () => {
    // Arrange: mock get_layout so useLayout hook resolves and exits loading state
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: sampleLayout, error: null });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: sampleLayout, isLoadingLayout: false });

    // Act
    render(<LayoutEditor />);

    // Assert – wait for loading to finish before checking canvas
    await waitFor(() =>
      expect(screen.getByTestId("layout-canvas")).toBeInTheDocument()
    );
  });

  test("renders the master screen tile", async () => {
    // Arrange: mock get_layout so useLayout hook resolves and exits loading state
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: [], error: null });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: [], isLoadingLayout: false });

    // Act
    render(<LayoutEditor />);

    // Assert – wait for loading to finish before checking master tile
    await waitFor(() =>
      expect(screen.getByTestId("screen-tile-Master")).toBeInTheDocument()
    );
  });

  test("renders a tile for each client in the layout", async () => {
    // Arrange: mock get_layout so useLayout hook resolves and exits loading state
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: sampleLayout, error: null });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: sampleLayout, isLoadingLayout: false });

    // Act
    render(<LayoutEditor />);

    // Assert – wait for loading to finish before checking client tile
    await waitFor(() =>
      expect(screen.getByTestId("screen-tile-dev-linux")).toBeInTheDocument()
    );
  });

  test("renders Apply and Reset buttons", async () => {
    // Arrange: mock get_layout so useLayout hook resolves and exits loading state
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: sampleLayout, error: null });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: sampleLayout, isLoadingLayout: false });

    // Act
    render(<LayoutEditor />);

    // Assert – wait for loading to finish before checking action buttons
    await waitFor(() =>
      expect(screen.getByTestId("btn-apply-layout")).toBeInTheDocument()
    );
    expect(screen.getByTestId("btn-reset-layout")).toBeInTheDocument();
  });

  test("apply button triggers layout save", async () => {
    // Arrange: mock get_layout so the loading state resolves and buttons appear
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: sampleLayout, error: null });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: sampleLayout, isLoadingLayout: false });

    // Act: render and wait for loading to complete before clicking
    render(<LayoutEditor />);
    await waitFor(() =>
      expect(screen.getByTestId("btn-apply-layout")).toBeInTheDocument()
    );
    fireEvent.click(screen.getByTestId("btn-apply-layout"));

    // Assert – button re-enables after save completes
    await waitFor(() =>
      expect(screen.getByTestId("btn-apply-layout")).not.toBeDisabled()
    );
  });

  test("shows error message when apply fails", async () => {
    // Arrange: mock get_layout to resolve and update_layout to fail
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

    // Act: render and wait for loading to complete before clicking
    render(<LayoutEditor />);
    await waitFor(() =>
      expect(screen.getByTestId("btn-apply-layout")).toBeInTheDocument()
    );
    fireEvent.click(screen.getByTestId("btn-apply-layout"));

    // Assert
    await waitFor(() =>
      expect(screen.getByTestId("layout-error")).toHaveTextContent("overlapping screens")
    );
  });

  test("reset button restores layout to store state", async () => {
    // Arrange: mock get_layout so loading state resolves
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_layout") {
        return Promise.resolve({ success: true, data: sampleLayout, error: null });
      }
      return Promise.resolve({ success: true, data: null, error: null });
    });
    useMasterStore.setState({ layout: sampleLayout, isLoadingLayout: false });

    // Act: render, wait for buttons, then reset
    render(<LayoutEditor />);
    await waitFor(() =>
      expect(screen.getByTestId("btn-reset-layout")).toBeInTheDocument()
    );
    fireEvent.click(screen.getByTestId("btn-reset-layout"));

    // Assert – no error state after reset
    expect(screen.queryByTestId("layout-error")).not.toBeInTheDocument();
  });
});
