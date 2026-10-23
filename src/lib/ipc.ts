// 与 Rust 侧命令的桥接层。
// 前端只通过这里调用后端；不直接碰 SQLite，不持有 API Key。

import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import type {
  AiConfigInput,
  AiSuggestion,
  AppInfo,
  Card,
  Category,
  FolderChild,
  ImportRequest,
  ImportResult,
  Settings,
} from "./types";

export const api = {
  // ---- 卡片 ----
  listCards: () => invoke<Card[]>("list_cards"),
  getCard: (id: string) => invoke<Card | null>("get_card", { id }),
  setSummary: (id: string, summary: string, aiGenerated: boolean) =>
    invoke<Card>("set_summary", { id, summary, aiGenerated }),
  setDisplayName: (id: string, displayName: string) =>
    invoke<Card>("set_display_name", { id, displayName }),
  setCategory: (id: string, category: string) =>
    invoke<Card>("set_category", { id, category }),
  setPinned: (id: string, pinned: boolean) =>
    invoke<Card>("set_pinned", { id, pinned }),
  relinkCard: (id: string, newPath: string) =>
    invoke<Card>("relink_card", { id, newPath }),
  deleteCard: (id: string) => invoke<void>("delete_card", { id }),

  // ---- 分类 ----
  listCategories: () => invoke<Category[]>("list_categories"),
  createCategory: (name: string) =>
    invoke<Category>("create_category", { name }),
  deleteCategory: (id: string) => invoke<void>("delete_category", { id }),

  // ---- 文件 ----
  openFile: (path: string) => invoke<void>("open_file", { path }),
  revealInFolder: (path: string) => invoke<void>("reveal_in_folder", { path }),
  listFolderChildren: (path: string) =>
    invoke<FolderChild[]>("list_folder_children", { path }),

  // ---- 导入 ----
  importPaths: (request: ImportRequest) =>
    invoke<ImportResult>("import_paths", { request }),
  folderChildren: (path: string) =>
    invoke<FolderChild[]>("folder_children", { path }),
  /** 取走启动时收到的待导入路径（右键「发送到 循迹」传入）。 */
  takePendingImports: () => invoke<string[]>("take_pending_imports"),

  // ---- 设置 ----
  getSettings: () => invoke<Settings>("get_settings"),
  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  saveSettings: (settings: Settings) =>
    invoke<Settings>("save_settings", { settings }),
  setAiConfig: (config: AiConfigInput) =>
    invoke<Settings>("set_ai_config", { config }),
  hasApiKey: () => invoke<boolean>("has_api_key"),
  clearApiKey: () => invoke<void>("clear_api_key"),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) =>
    invoke<boolean>("set_autostart", { enabled }),

  // ---- AI ----
  analyzeCard: (cardId: string) =>
    invoke<AiSuggestion>("analyze_card", { cardId }),
  analyzePath: (path: string) =>
    invoke<AiSuggestion>("analyze_path", { path }),
  testAiConnection: () => invoke<string>("test_ai_connection"),

  // ---- 系统 ----
  installShellMenu: () => invoke<string>("install_shell_menu"),
  uninstallShellMenu: () => invoke<string>("uninstall_shell_menu"),
  shellMenuInstalled: () => invoke<boolean>("shell_menu_installed"),
};

/** 把本地文件路径转成 WebView 可显示的 URL——用于展示缩略图。 */
export function thumbUrl(path: string | null): string | null {
  if (!path) return null;
  return convertFileSrc(path);
}

/** 把字节数转成人类可读的体积。 */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i += 1;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

/** 从 ISO 时间戳取出日期部分（时间线分组用）。 */
export function dateKey(iso: string): string {
  return iso.slice(0, 10);
}

/** 把日期部分格式化为「2026年10月22日」。 */
export function formatDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return `${d.getFullYear()}年${d.getMonth() + 1}月${d.getDate()}日`;
}

/** 把时间部分格式化为「14:32」。 */
export function formatTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  return `${hh}:${mm}`;
}
