import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import "./styles.css";

import { invoke } from "@tauri-apps/api/core";
const report = (message: string) => void invoke("debug_log", { message }).catch(() => {});
window.addEventListener("error", (event) => report(`error ${event.message}`));
window.addEventListener("unhandledrejection", (event) => report(`rejection ${String(event.reason)}`));
window.addEventListener("click", (event) => report(`click ${(event.target as HTMLElement)?.className}`), true);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
