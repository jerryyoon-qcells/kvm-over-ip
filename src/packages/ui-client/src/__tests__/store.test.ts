import { useClientStore } from "../store";
import type { ClientStatusDto, ClientSettingsDto } from "../types";

afterEach(() => {
  useClientStore.setState({
    status: null,
    settings: { masterAddress: "", clientName: "" },
    isLoading: false,
    lastError: null,
  });
});

describe("useClientStore", () => {
  test("setStatus stores the status", () => {
    // Arrange
    const status: ClientStatusDto = {
      connectionStatus: "Active",
      masterAddress: "192.168.1.1",
      clientName: "laptop",
      monitorCount: 1,
    };

    // Act
    useClientStore.getState().setStatus(status);

    // Assert
    expect(useClientStore.getState().status).toEqual(status);
  });

  test("setSettings stores the settings", () => {
    // Arrange
    const settings: ClientSettingsDto = {
      masterAddress: "10.0.0.1",
      clientName: "work-pc",
    };

    // Act
    useClientStore.getState().setSettings(settings);

    // Assert
    expect(useClientStore.getState().settings).toEqual(settings);
  });

  test("setLoading updates the loading flag", () => {
    // Act
    useClientStore.getState().setLoading(true);

    // Assert
    expect(useClientStore.getState().isLoading).toBe(true);
  });

  test("setLastError stores the error string", () => {
    // Act
    useClientStore.getState().setLastError("test error");

    // Assert
    expect(useClientStore.getState().lastError).toBe("test error");
  });

  test("setLastError clears the error when null is passed", () => {
    // Arrange
    useClientStore.getState().setLastError("some error");

    // Act
    useClientStore.getState().setLastError(null);

    // Assert
    expect(useClientStore.getState().lastError).toBeNull();
  });

  test("initial status is null", () => {
    expect(useClientStore.getState().status).toBeNull();
  });

  test("initial loading flag is false", () => {
    expect(useClientStore.getState().isLoading).toBe(false);
  });
});
