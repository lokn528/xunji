import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";
import { api } from "../lib/ipc";

/**
 * 自绘标题栏。
 *
 * Windows 上 Tauri 的 titleBarStyle / hiddenTitle 是 macOS 专属配置，不生效，
 * 原生那条深色标题栏去不掉。所以窗口用 decorations:false 起（无边框），
 * 这一条由前端自己画。
 *
 * 关键：整条不设独立底色，直接坐在应用的夜景背景上，留出的拖拽区透明可见，
 * 这样顶栏和应用背景连成一片，不再出现一条异色横条。
 */
export default function TitleBar() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    let cancelled = false;

    try {
      const win = getCurrentWindow();
      const sync = async () => {
        try {
          const isMax = await win.isMaximized();
          if (!cancelled) setMaximized(isMax);
        } catch {
          /* 窗口还没就绪，忽略 */
        }
      };
      void sync();
      void win
        .onResized(() => {
          void sync();
        })
        .then((unlisten) => {
          if (cancelled) unlisten();
          else dispose = unlisten;
        })
        .catch(() => undefined);
    } catch {
      /* 不在 Tauri 运行时里，忽略 */
    }

    return () => {
      cancelled = true;
      dispose?.();
    };
  }, []);

  const run = (fn: () => Promise<void>) => () => {
    fn().catch(() => undefined);
  };

  return (
    <div className="titlebar" data-tauri-drag-region="deep">
      <div className="win-btns">
        <button
          className="win-btn"
          type="button"
          aria-label="隐藏到系统托盘"
          title="隐藏到系统托盘"
          data-tauri-drag-region="false"
          onClick={run(api.hideToTray)}
        >
          <Minus size={15} strokeWidth={1.7} />
        </button>
        <button
          className="win-btn"
          type="button"
          aria-label={maximized ? "还原" : "最大化"}
          title={maximized ? "还原" : "最大化"}
          data-tauri-drag-region="false"
          onClick={run(() => getCurrentWindow().toggleMaximize())}
        >
          {maximized ? (
            <Copy size={13} strokeWidth={1.7} />
          ) : (
            <Square size={13} strokeWidth={1.7} />
          )}
        </button>
        <button
          className="win-btn win-btn-close"
          type="button"
          aria-label="关闭到系统托盘"
          title="关闭到系统托盘"
          data-tauri-drag-region="false"
          onClick={run(api.hideToTray)}
        >
          <X size={15} strokeWidth={1.7} />
        </button>
      </div>
    </div>
  );
}
