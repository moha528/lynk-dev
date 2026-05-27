import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";

/**
 * Suppress the default browser/webview context menu app-wide.
 *
 * Components that opt in to a custom context menu (Radix `ContextMenuTrigger`,
 * native `<input>`/`<textarea>` controls) call `preventDefault` themselves,
 * which sets `defaultPrevented` on the bubbling native event. We only suppress
 * the menu when nobody else has claimed the event — that way text fields keep
 * their native cut/copy/paste, and our custom right-click menus still open.
 */
document.addEventListener("contextmenu", (e) => {
  if (e.defaultPrevented) return;
  const target = e.target as HTMLElement | null;
  if (target?.closest('input, textarea, [contenteditable="true"]')) return;
  e.preventDefault();
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
