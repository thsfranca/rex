import * as React from "react";
import { createRoot } from "react-dom/client";

import themeCss from "./theme/themeVars.css";

import { App } from "./App";

const rootEl = document.getElementById("rex-root");
if (rootEl === null) {
  throw new Error("Missing #rex-root in webview shell");
}

injectStyles(themeCss);

createRoot(rootEl).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

function injectStyles(css: string): void {
  const style = document.createElement("style");
  style.setAttribute("data-rex-style", "true");
  style.textContent = css;
  document.head.appendChild(style);
}
