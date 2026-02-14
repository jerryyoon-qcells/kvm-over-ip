/**
 * Jest manual mock for `@tauri-apps/api/core`.
 *
 * # Why this mock is needed (for beginners)
 *
 * Tauri's `invoke()` function communicates with the Rust backend through an
 * IPC bridge that only exists inside a running Tauri WebView.  When Jest runs
 * tests in a Node.js environment there is no WebView, so real `invoke()` calls
 * would fail immediately.
 *
 * Jest's *manual mock* mechanism automatically substitutes this file for the
 * real `@tauri-apps/api/core` module in all test files, because this file
 * lives at the path `src/__mocks__/@tauri-apps/api/core.ts` — mirroring the
 * module path under `__mocks__/`.
 *
 * # How tests use `invoke`
 *
 * In each test file that needs to control `invoke` responses:
 * ```ts
 * import { invoke } from "@tauri-apps/api/core";
 * const mockInvoke = invoke as jest.MockedFunction<typeof invoke>;
 * mockInvoke.mockResolvedValue({ success: true, data: [...], error: null });
 * ```
 *
 * Between tests, `mockInvoke.mockReset()` clears the recorded calls and
 * return values so each test starts clean.
 */

// Mock for @tauri-apps/api/core used in Jest tests.
// Prevents tests from attempting real IPC calls.
export const invoke = jest.fn().mockResolvedValue(null);
