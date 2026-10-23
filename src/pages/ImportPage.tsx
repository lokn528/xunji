import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { File, FolderOpen, Sparkles, Trash2, X } from "lucide-react";
import { api } from "../lib/ipc";
import type {
  AiUsage,
  Category,
  ImportResult,
  Origin,
  Settings,
} from "../lib/types";
import { errorText } from "../lib/types";
import AiUsageLine from "../components/AiUsageLine";
import CategoryPicker from "../components/CategoryPicker";
import { useToast } from "../components/Toast";
import Switch from "../components/Switch";

function pathKey(path: string): string {
  return path.replace(/\//g, "\\").toLowerCase();
}

function fileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

function mergePaths(current: string[], incoming: string[]): string[] {
  const seen = new Set(current.map(pathKey));
  const next = [...current];
  for (const path of incoming) {
    const key = pathKey(path);
    if (!path.trim() || seen.has(key)) continue;
    seen.add(key);
    next.push(path);
  }
  return next;
}

export default function ImportPage({
  settings,
  categories,
  onSettingsChange,
  onCategoriesChange,
  onImported,
  pendingPaths,
  onPendingDone,
  autoAi,
  onAutoAiChange,
  onNavigateSettings,
}: {
  settings: Settings | null;
  categories: Category[];
  onSettingsChange: (s: Settings) => void;
  onCategoriesChange: (categories: Category[]) => void;
  onImported: () => void;
  /** 启动时收到的待导入路径（右键「发送到 循迹」）。 */
  pendingPaths: string[] | null;
  onPendingDone: () => void;
  autoAi: boolean;
  onAutoAiChange: (enabled: boolean) => void;
  onNavigateSettings: () => void;
}) {
  const toast = useToast();
  const [over, setOver] = useState(false);
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<ImportResult | null>(null);
  const [stagedPaths, setStagedPaths] = useState<string[]>([]);
  const [targetDir, setTargetDir] = useState("");
  const [useTempDir, setUseTempDir] = useState(false);
  const [placement, setPlacement] = useState<"move" | "keep">("move");
  const [selectedCategories, setSelectedCategories] = useState<string[]>([]);
  const [suggestedCategories, setSuggestedCategories] = useState<string[]>([]);
  const [note, setNote] = useState("");
  const [keyPresent, setKeyPresent] = useState<boolean | null>(null);
  const [aiFilling, setAiFilling] = useState(false);
  const [aiUsage, setAiUsage] = useState<AiUsage | null>(null);
  const [showAiSetup, setShowAiSetup] = useState(false);
  const latestStagePaths = useRef<(paths: string[]) => void>(() => undefined);

  const aiConfigured =
    settings && keyPresent !== null
      ? Boolean(
          keyPresent &&
            settings.aiBaseUrl.trim() &&
            settings.aiModel.trim(),
        )
      : null;

  useEffect(() => {
    if (settings) {
      setTargetDir(settings.defaultDir);
      setPlacement(settings.placementMode);
    }
  }, [settings]);

  useEffect(() => {
    let alive = true;
    api
      .hasApiKey()
      .then((present) => {
        if (alive) setKeyPresent(present);
      })
      .catch(() => {
        if (alive) setKeyPresent(false);
      });
    return () => {
      alive = false;
    };
  }, []);

  useEffect(() => {
    if (autoAi && aiConfigured === false) onAutoAiChange(false);
  }, [autoAi, aiConfigured, onAutoAiChange]);

  const fillWithAi = async (paths: string[]) => {
    if (paths.length === 0) return;
    if (aiConfigured !== true) {
      setShowAiSetup(true);
      return;
    }

    setAiFilling(true);
    setAiUsage(null);
    try {
      const suggestion = await api.analyzePath(paths[0]);
      setNote(suggestion.summary);
      setSuggestedCategories(suggestion.suggestedCategories);
      setAiUsage(suggestion.usage ?? null);
      setSelectedCategories((current) => [
        ...new Set([...current, ...suggestion.suggestedCategories]),
      ]);
      toast.ok(
        "AI 已填入备注并选中建议分类",
        paths.length > 1 ? "按第一个文件生成，可在导入前继续修改" : undefined,
      );
    } catch (e) {
      setAiUsage(null);
      toast.err("AI 填充失败，可手动填写", errorText(e));
    } finally {
      setAiFilling(false);
    }
  };

  const stagePaths = (paths: string[]) => {
    const next = mergePaths(stagedPaths, paths);
    setStagedPaths(next);
    setSuggestedCategories([]);
    setAiUsage(null);
    if (autoAi && aiConfigured === true && next.length > 0) {
      void fillWithAi(next);
    }
  };
  latestStagePaths.current = stagePaths;

  const doImport = async (paths: string[], origin?: Origin) => {
    if (paths.length === 0) return;
    setBusy(true);
    try {
      const result = await api.importPaths({
        paths,
        targetDir: useTempDir && targetDir.trim() ? targetDir : null,
        placement,
        analyzeAfter: false,
        categories: selectedCategories,
        note: note || null,
        origin: origin ?? null,
      });
      setReport(result);
      setStagedPaths(
        result.items.filter((item) => item.error).map((item) => item.sourcePath),
      );
      if (result.succeeded > 0) {
        setNote("");
        setSelectedCategories([]);
        setSuggestedCategories([]);
        setAiUsage(null);
      }

      if (result.failed > 0) {
        const firstError = result.items.find((item) => item.error)?.error;
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

  // Tauri 会接管 Windows 的原生文件拖放，因此必须监听 WebView 事件，
  // 不能依赖浏览器 DataTransfer.files。
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          setOver(true);
        } else if (event.payload.type === "leave") {
          setOver(false);
        } else if (event.payload.type === "drop") {
          setOver(false);
          latestStagePaths.current(event.payload.paths);
        }
      })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch((e) => {
        toast.err("无法接收拖入文件", errorText(e));
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [toast]);

  // 启动时收到待导入路径（右键「发送到 循迹」）→ 自动导入一次。
  useEffect(() => {
    if (!pendingPaths || pendingPaths.length === 0) return;
    const paths = pendingPaths;
    onPendingDone();
    void doImport(paths, "sent_to");
    // 只在拿到待导入路径时触发一次
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pendingPaths]);

  const pickFiles = async () => {
    const picked = await open({
      multiple: true,
      directory: false,
      title: "选择要收录的文件",
    });
    if (!picked) return;
    stagePaths(Array.isArray(picked) ? picked : [picked]);
  };

  const pickFolder = async () => {
    const picked = await open({
      multiple: false,
      directory: true,
      title: "选择要收录的文件夹",
    });
    if (!picked || Array.isArray(picked)) return;
    stagePaths([picked]);
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

  return (
    <>
      <div className="page-head page-head-solo">
        <h2>导入</h2>
      </div>

      <div className={`dropzone${over ? " over" : ""}`} aria-busy={busy}>
        <File className="big" size={34} aria-hidden="true" />
        <h3>{over ? "松手加入待导入" : "把文件拖到这里"}</h3>
        <p>
          文件与文件夹都收。图片生成缩略图；TXT、Markdown、代码等文本可读取开头；
          Word、PDF 等按文件信息收录。
        </p>
        <div className="row dropzone-actions">
          <button className="btn btn-primary" onClick={pickFiles}>
            选择文件
          </button>
          <button className="btn" onClick={pickFolder}>
            选择文件夹
          </button>
        </div>
      </div>

      {stagedPaths.length > 0 && (
        <section className="staged-panel">
          <header>
            <div>
              <strong>待导入</strong>
              <span className="badge badge-plain">{stagedPaths.length} 项</span>
            </div>
            <button
              className="btn-icon"
              type="button"
              onClick={() => {
                setStagedPaths([]);
                setAiUsage(null);
              }}
              disabled={busy}
              title="清空待导入列表"
              aria-label="清空待导入列表"
            >
              <Trash2 size={15} />
            </button>
          </header>
          <ul>
            {stagedPaths.map((path) => (
              <li key={pathKey(path)}>
                <File size={15} aria-hidden="true" />
                <span className="name" title={path}>
                  {fileName(path)}
                </span>
                <button
                  className="btn-icon"
                  type="button"
                  onClick={() => {
                    setStagedPaths((current) =>
                      current.filter((item) => pathKey(item) !== pathKey(path)),
                    );
                    setAiUsage(null);
                  }}
                  disabled={busy}
                  title="移出待导入列表"
                  aria-label={`移除 ${fileName(path)}`}
                >
                  <X size={14} />
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}

      <div className="import-settings">
        <div className="import-settings-grid">
          <label className="field import-location">
            <span>放到哪里</span>
            <div className="row">
              <input
                className="input mono"
                value={targetDir}
                onChange={(e) => {
                  setTargetDir(e.target.value);
                  setUseTempDir(true);
                }}
                placeholder="默认存储位置"
              />
              <button className="btn" onClick={chooseTargetDir} type="button">
                <FolderOpen size={15} />
                选目录
              </button>
            </div>
          </label>

          <label className="field">
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

          <div className="field import-category">
            <span>分类</span>
            <CategoryPicker
              categories={categories}
              selected={selectedCategories}
              suggested={suggestedCategories}
              onChange={setSelectedCategories}
              onCategoriesChange={onCategoriesChange}
              disabled={busy}
            />
          </div>

          <label className="field import-note">
            <span>备注说明</span>
            <textarea
              className="textarea"
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder="可留空，也可以让 AI 先填一版"
              rows={2}
            />
          </label>
        </div>

        <div className="import-actions">
          <div className="row row-wrap">
            {useTempDir && (
              <button className="btn btn-sm" onClick={saveAsDefault} type="button">
                存为默认
              </button>
            )}
            <button
              className="btn"
              type="button"
              onClick={() => void fillWithAi(stagedPaths)}
              disabled={stagedPaths.length === 0 || aiFilling || busy}
            >
              <Sparkles size={15} />
              {aiFilling ? "AI 分析中…" : "AI 智能填充"}
            </button>
          </div>
          <span className="spacer" />
          <button
            className="btn btn-primary import-submit"
            type="button"
            onClick={() => void doImport(stagedPaths)}
            disabled={stagedPaths.length === 0 || busy}
          >
            {busy ? "正在导入…" : "导入"}
          </button>
        </div>

        <AiUsageLine usage={aiUsage} />

        <div className="auto-ai-row">
          <div className="auto-ai-copy">
            <Sparkles size={16} aria-hidden="true" />
            <div>
              <strong>自动 AI 填充</strong>
              <span>加入文件后，自动填备注并选中分类建议，可再增删。</span>
            </div>
          </div>
          <Switch
            checked={autoAi}
            onChange={(enabled) => {
              if (enabled && aiConfigured !== true) {
                setShowAiSetup(true);
                return;
              }
              onAutoAiChange(enabled);
              if (enabled && stagedPaths.length > 0) {
                void fillWithAi(stagedPaths);
              }
            }}
            disabled={aiConfigured === null || busy}
            label="自动 AI 填充"
          />
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
              <li key={`${item.sourcePath}-${i}`}>
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

      {showAiSetup && (
        <div className="modal-backdrop" onClick={() => setShowAiSetup(false)}>
          <div className="modal modal-compact" onClick={(e) => e.stopPropagation()}>
            <header>
              <Sparkles size={18} aria-hidden="true" />
              <h3>先配置 AI</h3>
              <button
                className="btn-icon"
                type="button"
                onClick={() => setShowAiSetup(false)}
                aria-label="关闭"
              >
                <X size={15} />
              </button>
            </header>
            <div className="body">
              <p className="modal-lead">
                自动分析或手动填充需要 Base URL、模型名和 API Key。配置一次后会保存在本机。
              </p>
              <div className="modal-note">
                API Key 存入 Windows 凭据库，不会写进数据库。
              </div>
            </div>
            <footer>
              <button className="btn" onClick={() => setShowAiSetup(false)}>
                稍后
              </button>
              <button
                className="btn btn-primary"
                onClick={() => {
                  setShowAiSetup(false);
                  onNavigateSettings();
                }}
              >
                去设置
              </button>
            </footer>
          </div>
        </div>
      )}
    </>
  );
}
