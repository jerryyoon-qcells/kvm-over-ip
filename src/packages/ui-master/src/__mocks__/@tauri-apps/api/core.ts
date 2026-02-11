// Mock for @tauri-apps/api/core used in Jest tests.
// Prevents tests from attempting real IPC calls.

export const invoke = jest.fn().mockResolvedValue(null);
