import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/ipc";
import type { ImportResult, Origin, Settings } from "../lib/types";
import { errorText } from "../lib/types";
import { useToast } from "../components/Toast";

export default function ImportPage({
  settings,
  onSettingsChange,
  onImported,
  pendingPaths,
  onPendingDone,
}: {
  settings: Settings | null;
  onSettingsChange: (s: Settings) => void;
  onImported: () => void;
  /** 启动时收到的待导入路径（右键「发送到 循迹」）。 */
  pendingPaths: string[] | null;
  onPendingDone: () => void;
}) {
  const toast = useToast();
  const [over, setOver] = useState(false);
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<ImportResult | null>(null);
  const [targetDir, setTargetDir] = useState("");
  const [useTempDir, setUseTempDir] = useState(false);
  const [analyzeAfter, setAnalyzeAfter] = useState(false);
  const [placement, setPlacement] = useState<"move" | "keep">("move");
  const dragDepth = useRef(0);

  useEffect(() => {
    if (settings) {
      setTargetDir(settings.defaultDir);
      setPlacement(settings.placementMode);
    }
  }, [settings]);

  const doImport = async (paths: string[], origin?: Origin) => {
    if (paths.length === 0) return;
    setBusy(true);
    try {
      const result = await api.importPaths({
        paths,
        targetDir: useTempDir && targetDir.trim() ? targetDir : null,
        placement,
        analyzeAfter,
        origin: origin ?? null,
      });
      setReport(result);

      if (result.failed > 0) {
        const firstError = result.items.find((i) => i.error)?.error;
        toast.err(`${result.failed} 个没能收录`, firstError);
      } else if (result.duplicates > 0) {
        toast.ok(`收录 ${result.succeeded} 个，${result.duplicates} 个已收录过`);
      } else {
        toast.ok(`收录 ${result.succeeded} 个文件`);
      }
      onImported();
    } catch (e) {
      toast.err("导入失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  // 启动时收到待导入路径（右键「发送到 循迹」）→ 自动导入一次。
  useEffect(() => {
    if (!pendingPaths || pendingPaths.length === 0) return;
    const paths = pendingPaths;
    onPendingDone();
    void doImport(paths, "sent_to");
    // 只在拿到待导入路径时触发一次
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pendingPaths]);

  const onDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    dragDepth.current = 0;
    setOver(false);
    const paths: string[] = [];
    for (const file of Array.from(e.dataTransfer.files)) {
      const p = (file as File & { path?: string }).path;
      if (p) paths.push(p);
    }
    if (paths.length === 0) {
      toast.err("读不到拖入文件的路径", "试试用「选择文件」按钮");
      return;
    }
    await doImport(paths);
  };

  const pickFiles = async () => {
    const picked = await open({
      multiple: true,
      directory: false,
      title: "选择要收录的文件",
    });
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];
    await doImport(paths);
  };

  const pickFolder = async () => {
    const picked = await open({
      multiple: false,
      directory: true,
      title: "选择要收录的文件夹",
    });
    if (!picked || Array.isArray(picked)) return;
    await doImport([picked]);
  };

  const chooseTargetDir = async () => {
    const picked = await open({
      multiple: false,
      directory: true,
      title: "选择存储位置",
    });
    if (!picked || Array.isArray(picked)) return;
    setTargetDir(picked);
    setUseTempDir(true);
  };

  const saveAsDefault = async () => {
    if (!settings) return;
    try {
      const next = await api.saveSettings({
        ...settings,
        defaultDir: targetDir,
        placementMode: placement,
      });
      onSettingsChange(next);
      setUseTempDir(false);
      toast.ok("已设为默认存储位置");
    } catch (e) {
      toast.err("保存失败", errorText(e));
    }
  };

  const noDefault = !settings?.defaultDir?.trim();

  return (
    <>
      <div className="page-head">
        <h2>导入</h2>
        <p>
          把丢在一旁的文件丢进来，给它发一张户口卡。
          {noDefault && " 先到设置里选一个默认存储位置。"}
        </p>
      </div>

      <div
        className={`dropzone${over ? " over" : ""}`}
        onDragEnter={(e) => {
          e.preventDefault();
          dragDepth.current += 1;
          setOver(true);
        }}
        onDragOver={(e) => e.preventDefault()}
        onDragLeave={(e) => {
          e.preventDefault();
          dragDepth.current -= 1;
          if (dragDepth.current <= 0) setOver(false);
        }}
        onDrop={onDrop}
      >
        <div className="big" aria-hidden="true">
          ⇩
        </div>
        <h3>把文件拖到这里</h3>
        <p>文件与文件夹都收。文件夹整体建一张卡，可以展开看子项。</p>
        <div className="row" style={{ justifyContent: "center" }}>
          <button className="btn btn-primary" onClick={pickFiles} disabled={busy}>
            选择文件
          </button>
          <button className="btn" onClick={pickFolder} disabled={busy}>
            选择文件夹
          </button>
        </div>
      </div>

      <div className="import-bar">
        <label className="field">
          <span>这次放到哪</span>
          <div className="row">
            <input
              className="input mono"
              value={targetDir}
              onChange={(e) => {
                setTargetDir(e.target.value);
                setUseTempDir(true);
              }}
              placeholder="默认存储位置（到设置里改）"
            />
            <button className="btn" onClick={chooseTargetDir}>
              选目录
            </button>
          </div>
        </label>

        <label className="field" style={{ flex: "0 0 auto" }}>
          <span>落点方式</span>
          <select
            className="select"
            value={placement}
            onChange={(e) => setPlacement(e.target.value as "move" | "keep")}
          >
            <option value="move">移动/复制到目标位置</option>
            <option value="keep">仅登记，原地不动</option>
          </select>
        </label>

        <div className="field" style={{ flex: "0 0 auto" }}>
          <span>&nbsp;</span>
          <div className="row">
            <label className="switch-row" style={{ padding: 0 }}>
              <input
                type="checkbox"
                checked={analyzeAfter}
                onChange={(e) => setAnalyzeAfter(e.target.checked)}
              />
              <span>导入后立即 AI 分析</span>
            </label>
            {useTempDir && (
              <button className="btn btn-sm" onClick={saveAsDefault}>
                存为默认
              </button>
            )}
          </div>
        </div>
      </div>

      {report && (
        <div className="import-report">
          <header>
            <span className="badge badge-ok">成功 {report.succeeded}</span>
            {report.duplicates > 0 && (
              <span className="badge">已收录 {report.duplicates}</span>
            )}
            {report.failed > 0 && (
              <span className="badge badge-danger">失败 {report.failed}</span>
            )}
          </header>
          <ul>
            {report.items.map((item, i) => (
              <li key={i}>
                {item.error ? (
                  <span className="badge badge-danger">失败</span>
                ) : item.existingCardId ? (
                  <span className="badge">已收录</span>
                ) : (
                  <span className="badge badge-ok">已建卡</span>
                )}
                <span className="path" title={item.sourcePath}>
                  {item.sourcePath}
                </span>
                {item.error && <span className="muted">{item.error}</span>}
              </li>
            ))}
          </ul>
        </div>
      )}
    </>
  );
}
