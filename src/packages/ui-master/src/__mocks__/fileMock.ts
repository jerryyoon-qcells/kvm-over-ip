/**
 * Jest file mock for static assets (CSS, images, SVGs, fonts, etc.).
 *
 * # Why this mock is needed (for beginners)
 *
 * Vite handles non-JavaScript imports (like `import "./App.css"` or
 * `import logo from "./logo.svg"`) during the build.  Jest runs in Node.js
 * and cannot parse these files natively — it would throw a syntax error on
 * the first CSS or image import it encounters.
 *
 * The Jest configuration (`jest.config.ts`) maps static-asset file extensions
 * to this mock using the `moduleNameMapper` option:
 * ```ts
 * moduleNameMapper: {
 *   "\\.(css|less|svg|png|jpg|gif|woff|woff2)$":
 *     "<rootDir>/src/__mocks__/fileMock.ts",
 * }
 * ```
 *
 * This mock exports `{}` (an empty object), so:
 * - CSS imports are silently ignored during tests (no styles applied).
 * - Image/font imports evaluate to `{}` rather than a real URL.
 *
 * Tests should not rely on styles being applied.  Use `data-testid` attributes
 * and accessible roles for DOM queries instead of CSS class names.
 */

// Stub for CSS module and static-asset imports during Jest tests.
export default {};
