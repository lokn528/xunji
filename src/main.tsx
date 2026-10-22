import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import NotInTauri from "./components/NotInTauri";
import "./styles/global.css";

// 循迹依赖 Tauri 运行时（本地 SQLite、Rust 侧文件操作、系统凭据库）。
// 不在 Tauri 里时给出说明，而不是一片空白。
const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {inTauri ? <App /> : <NotInTauri />}
  </React.StrictMode>,
);
