import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Cable, MousePointer2, Power, Sparkles } from "lucide-react";
import { api } from "../lib/ipc";
import type { Settings } from "../lib/types";
import { errorText } from "../lib/types";
import { useToast } from "../components/Toast";
import Switch from "../components/Switch";

interface AppInfo {
  dataDir: string;
  dbPath: string;
  thumbDir: string;
  exePath: string;
  version: string;
}

const PROVIDER_PRESETS = {
  openai_compatible: { label: "自定义 / 中转站", baseUrl: "", model: "" },
  openai: {
    label: "OpenAI 官方",
    baseUrl: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
  },
  deepseek: {
    label: "DeepSeek",
    baseUrl: "https://api.deepseek.com/v1",
    model: "deepseek-chat",
  },
  zhipu: {
    label: "智谱 GLM",
    baseUrl: "https://open.bigmodel.cn/api/paas/v4",
    model: "glm-4-flash",
  },
  qwen: {
    label: "通义千问",
    baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    model: "qwen-plus",
  },
} as const;

export default function SettingsPage({
  settings,
  onSettingsChange,
}: {
  settings: Settings | null;
  onSettingsChange: (s: Settings) => void;
}) {
  const toast = useToast();
  const [defaultDir, setDefaultDir] = useState("");
  const [placementMode, setPlacementMode] = useState<"move" | "keep">("move");
  const [provider, setProvider] = useState("openai_compatible");
  const [baseUrl, setBaseUrl] = useState("");
  const [model, setModel] = useState("");
  const [temperature, setTemperature] = useState(0.3);
  const [filenameOnly, setFilenameOnly] = useState(true);
  const [apiKey, setApiKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [keyPresent, setKeyPresent] = useState(false);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [menuInstalled, setMenuInstalled] = useState(false);
  const [autostart, setAutostart] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!settings) return;
    setDefaultDir(settings.defaultDir);
    setPlacementMode(settings.placementMode);
    setProvider(settings.aiProvider);
    setBaseUrl(settings.aiBaseUrl);
    setModel(settings.aiModel);
    setTemperature(settings.aiTemperature);
    setFilenameOnly(settings.aiFilenameOnly);
  }, [settings]);

  useEffect(() => {
    api.hasApiKey().then(setKeyPresent).catch(() => setKeyPresent(false));
    api.getAppInfo().then(setInfo).catch(() => undefined);
    api.shellMenuInstalled?.().then(setMenuInstalled).catch(() => undefined);
    api.getAutostart().then(setAutostart).catch(() => setAutostart(false));
  }, []);

  const chooseDir = async () => {
    const picked = await open({
      multiple: false,
      directory: true,
      title: "选择默认存储位置",
    });
    if (!picked || Array.isArray(picked)) return;
    setDefaultDir(picked);
  };

  const changeProvider = (value: string) => {
    setProvider(value);
    const preset =
      PROVIDER_PRESETS[value as keyof typeof PROVIDER_PRESETS] ??
      PROVIDER_PRESETS.openai_compatible;
    if (value !== "openai_compatible") {
      setBaseUrl(preset.baseUrl);
      setModel(preset.model);
    }
  };

  const saveStorage = async () => {
    if (!settings) return;
    setBusy(true);
    try {
      const next = await api.saveSettings({
        ...settings,
        defaultDir,
        placementMode,
      });
      onSettingsChange(next);
      toast.ok("已保存存储设置");
    } catch (e) {
      toast.err("保存失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const saveAi = async () => {
    setBusy(true);
    try {
      const next = await api.setAiConfig({
        provider,
        baseUrl,
        apiKey,
        model,
        temperature,
        filenameOnly,
      });
      onSettingsChange(next);
      setApiKey("");
      setKeyPresent(await api.hasApiKey());
      toast.ok("已保存 AI 配置");
    } catch (e) {
      toast.err("保存失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const testAi = async () => {
    setBusy(true);
    try {
      const msg = await api.testAiConnection();
      toast.ok("连接正常", msg);
    } catch (e) {
      toast.err("连不上", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const clearKey = async () => {
    setBusy(true);
    try {
      await api.clearApiKey();
      setKeyPresent(false);
      toast.ok("已清除 API Key");
    } catch (e) {
      toast.err("清除失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const toggleMenu = async () => {
    setBusy(true);
    try {
      const msg = menuInstalled
        ? await api.uninstallShellMenu()
        : await api.installShellMenu();
      setMenuInstalled(!menuInstalled);
      toast.ok(msg);
    } catch (e) {
      toast.err("操作失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const toggleAutostart = async () => {
    setBusy(true);
    try {
      const next = await api.setAutostart(!autostart);
      setAutostart(next);
      toast.ok(next ? "已设为开机自启" : "已关闭开机自启");
    } catch (e) {
      toast.err("设置失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <div className="page-head">
        <h2>设置</h2>
        <p>文件留在本机，AI 密钥存进系统凭据库。</p>
      </div>

      <section className="panel">
        <h3>存储</h3>

        <label className="field">
          <span>默认存储位置</span>
          <div className="row">
            <input
              className="input mono"
              value={defaultDir}
              onChange={(e) => setDefaultDir(e.target.value)}
              placeholder="例如 D:\循迹库"
            />
            <button className="btn" onClick={chooseDir}>
              选目录
            </button>
          </div>
          <span className="hint">
            首次启动会自动使用程序目录下的 library，可改成常用的资料盘或网盘目录。
          </span>
        </label>

        <div className="field">
          <span>落点方式</span>
          <div className="radio-row">
            <label className="radio-card">
              <input
                type="radio"
                checked={placementMode === "move"}
                onChange={() => setPlacementMode("move")}
              />
              <span>
                <strong>移动/复制到默认位置</strong>
                <small>解决桌面堆积。默认。</small>
              </span>
            </label>
            <label className="radio-card">
              <input
                type="radio"
                checked={placementMode === "keep"}
                onChange={() => setPlacementMode("keep")}
              />
              <span>
                <strong>仅登记不移动</strong>
                <small>原地登记，文件不动。</small>
              </span>
            </label>
          </div>
        </div>

        <button className="btn btn-primary" onClick={saveStorage} disabled={busy}>
          保存存储设置
        </button>
      </section>

      <section className="panel">
        <h3>AI 供应商</h3>

        <div className="protocol-card">
          <div className="protocol-icon" aria-hidden="true">
            <Cable size={17} />
          </div>
          <div>
            <strong>接口协议：OpenAI Chat Completions</strong>
            <span>
              OpenAI 官方、DeepSeek、智谱、通义和多数中转站都兼容这个协议；
              Anthropic Messages 等原生接口暂未接入。
            </span>
          </div>
        </div>

        <label className="field">
          <span>供应商</span>
          <select
            className="select"
            value={provider}
            onChange={(e) => changeProvider(e.target.value)}
          >
            {Object.entries(PROVIDER_PRESETS).map(([value, preset]) => (
              <option key={value} value={value}>
                {preset.label}
              </option>
            ))}
          </select>
          <span className="hint">
            选官方供应商会自动填入常用的 Base URL 和模型名，也可以再改。
          </span>
        </label>

        <label className="field">
          <span>Base URL</span>
          <input
            className="input mono"
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
            placeholder="https://api.deepseek.com/v1"
          />
        </label>

        <label className="field">
          <span>模型名</span>
          <input
            className="input mono"
            value={model}
            onChange={(e) => setModel(e.target.value)}
            placeholder="deepseek-chat"
          />
        </label>

        <label className="field">
          <span>API Key</span>
          <div className="row">
            <input
              className="input mono"
              type={showKey ? "text" : "password"}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={keyPresent ? "已保存，留空表示不改动" : "尚未配置"}
              autoComplete="off"
            />
            <button
              className="btn"
              onClick={() => setShowKey(!showKey)}
              type="button"
            >
              {showKey ? "隐藏" : "显示"}
            </button>
            {keyPresent && (
              <button className="btn" onClick={clearKey} disabled={busy}>
                清除
              </button>
            )}
          </div>
          <span className="hint">
            存进 Windows 凭据库，不写进配置文件，数据文件里也搜不到。
          </span>
        </label>

        <label className="field">
          <span>温度 · {temperature.toFixed(1)}</span>
          <input
            type="range"
            min={0}
            max={1}
            step={0.1}
            value={temperature}
            onChange={(e) => setTemperature(Number(e.target.value))}
          />
          <span className="hint">
            控制 AI 的随机性：越低越稳定，越高越发散；资料整理建议保持 0.2～0.4。
          </span>
        </label>

        <div className="setting-toggle-row">
          <div className="setting-toggle-copy">
            <div className="setting-icon" aria-hidden="true">
              <Sparkles size={16} />
            </div>
            <div>
              <strong>仅用文件名分析</strong>
              <span>打开时只发送文件名与类型；关掉后文本文件会附上开头一段。</span>
            </div>
          </div>
          <Switch
            checked={filenameOnly}
            onChange={setFilenameOnly}
            label="仅用文件名分析"
          />
        </div>

        <div className="row">
          <button className="btn btn-primary" onClick={saveAi} disabled={busy}>
            保存 AI 配置
          </button>
          <button className="btn" onClick={testAi} disabled={busy}>
            测试连通性
          </button>
        </div>
      </section>

      <section className="panel">
        <h3>系统</h3>
        <div className="setting-toggle-row">
          <div className="setting-toggle-copy">
            <div className="setting-icon" aria-hidden="true">
              <MousePointer2 size={16} />
            </div>
            <div>
              <strong>资源管理器右键菜单</strong>
              <span>在文件和文件夹上显示「发送到 循迹」。</span>
            </div>
          </div>
          <Switch
            checked={menuInstalled}
            onChange={() => void toggleMenu()}
            disabled={busy}
            label="资源管理器右键菜单"
          />
        </div>

        <div className="setting-toggle-row">
          <div className="setting-toggle-copy">
            <div className="setting-icon" aria-hidden="true">
              <Power size={16} />
            </div>
            <div>
              <strong>开机自动启动</strong>
              <span>登录 Windows 后在后台启动循迹。</span>
            </div>
          </div>
          <Switch
            checked={autostart}
            onChange={() => void toggleAutostart()}
            disabled={busy}
            label="开机自动启动"
          />
        </div>

        {info && (
          <div style={{ marginTop: 16, fontSize: 12 }}>
            <div className="muted" style={{ marginBottom: 6 }}>
              数据与程序位置
            </div>
            <div className="mono" style={{ marginBottom: 3 }}>
              数据目录：{info.dataDir}
            </div>
            <div className="mono" style={{ marginBottom: 3 }}>
              数据库：{info.dbPath}
            </div>
            <div className="mono" style={{ marginBottom: 3 }}>
              缩略图缓存：{info.thumbDir}
            </div>
            <div className="mono">版本：{info.version}</div>
            <div className="hint" style={{ marginTop: 8 }}>
              备份就是复制数据库文件。
            </div>
          </div>
        )}
      </section>
    </>
  );
}
