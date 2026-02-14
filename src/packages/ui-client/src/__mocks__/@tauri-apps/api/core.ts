/**
 * Jest manual mock for `@tauri-apps/api/core`.
 *
 * # Why this mock is needed (for beginners)
 *
 * When Jest runs tests it uses Node.js, not the Tauri WebView runtime.
 * There is no real Tauri IPC bridge available in the test environment, so
 * any call to the real `invoke()` function would throw an error.
 *
 * This file is a *manual mock* — Jest's module resolution automatically
 * uses this file instead of the real `@tauri-apps/api/core` module whenever
 * a test file imports from that package.  The mock path must mirror the
 * module path under `src/__mocks__/` for this automatic substitution to work.
 *
 * # `jest.fn().mockResolvedValue(null)`
 *
 * `jest.fn()` creates a mock function that records every call.  Tests can:
 * - Override the return value with `.mockResolvedValue(data)`.
 * - Assert it was called with `expect(mockInvoke).toHaveBeenCalledWith(...)`.
 * - Reset it between tests with `mockInvoke.mockReset()`.
 *
 * The default return value of `null` means tests that don't set up an explicit
 * mock response will receive `null` (rather than throwing).
 */

// Exports a mock `invoke` function that does nothing by default.
// Individual tests override this with mockInvoke.mockResolvedValue(...).
export const invoke = jest.fn().mockResolvedValue(null);
