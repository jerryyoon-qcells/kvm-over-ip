/**
 * Application entry point for the kvm-master Tauri UI.
 *
 * This is the first file executed when the Tauri WebView loads the React app.
 * It mounts the root `<App />` component into the `#root` div defined in
 * `index.html`.
 *
 * # React.StrictMode (for beginners)
 *
 * Wrapping the app in `<React.StrictMode>` enables extra development-time
 * checks:
 * - Components are rendered twice (in development only) to detect side effects
 *   in render functions that should be pure.
 * - Deprecated lifecycle methods trigger warnings.
 * - Effects (`useEffect`) run an extra time to help detect missing cleanup
 *   functions.
 *
 * StrictMode has **no effect in production builds** — it is development-only.
 *
 * # The `!` non-null assertion
 *
 * `document.getElementById("root")` returns `HTMLElement | null`.  The `!`
 * asserts to TypeScript that the element is definitely present (it is, because
 * `index.html` defines `<div id="root" />`).  The ESLint disable comment
 * acknowledges that we are intentionally overriding the null-safety check.
 */

import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// Mount the React application tree into the root DOM node.
// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
