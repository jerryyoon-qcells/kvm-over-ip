/**
 * Jest file mock for static assets (CSS, images, SVGs, etc.).
 *
 * # Why this mock is needed (for beginners)
 *
 * Vite bundles CSS and image imports as part of the build process.  However,
 * Jest (which runs in Node.js, not a browser) does not know how to handle
 * non-JavaScript imports like `.css` files.
 *
 * When Jest encounters `import "./App.css"` or `import logo from "./logo.svg"`,
 * it would normally throw a syntax error because Node.js cannot parse CSS.
 *
 * The Jest configuration (`jest.config.ts`) maps these file extensions to this
 * mock module using the `moduleNameMapper` option:
 * ```ts
 * moduleNameMapper: {
 *   "\\.(css|less|svg|png|jpg)$": "<rootDir>/src/__mocks__/fileMock.ts",
 * }
 * ```
 *
 * This mock exports an empty object (`{}`).  CSS imports become `{}` (no styles
 * applied during tests, which is acceptable), and image imports become `{}`.
 *
 * Tests should not rely on CSS class names for queries; use `data-testid`
 * attributes or accessible roles instead (as this project does).
 */

// Return an empty object for any static asset import.
export default {};
