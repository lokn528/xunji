import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/ipc";
import type { Settings } from "../lib/types";
import { errorText } from "../lib/types";
import { useToast } from "../components/Toast";

interface AppInfo {
  dataDir: string;
  dbPath: string;
  thumbDir: string;
  exePath: string;
  version: string;
}

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
            导入时文件会落到这里。留空则只能在「仅登记」模式下导入。
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

        <label className="field">
          <span>供应商</span>
          <select
            className="select"
            value={provider}
            onChange={(e) => setProvider(e.target.value)}
          >
            <option value="openai_compatible">OpenAI 兼容（含中转站）</option>
            <option value="deepseek">DeepSeek</option>
            <option value="zhipu">智谱 GLM</option>
            <option value="qwen">通义千问</option>
            <option value="openai">OpenAI</option>
          </select>
          <span className="hint">都走 OpenAI 兼容的 /chat/completions 接口。</span>
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
        </label>

        <div className="switch-row">
          <input
            type="checkbox"
            id="filename-only"
            checked={filenameOnly}
            onChange={(e) => setFilenameOnly(e.target.checked)}
          />
          <label htmlFor="filename-only">
            仅用文件名分析（不发送文件内容）
            <span className="hint" style={{ marginTop: 2 }}>
              打开时只把文件名和类型发给 AI。关掉后文本类文件会附上开头一段。
            </span>
          </label>
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
        <div className="row row-wrap">
          <button className="btn" onClick={toggleMenu} disabled={busy}>
            {menuInstalled ? "移除右键「发送到 循迹」" : "注册右键「发送到 循迹」"}
          </button>
          <span className="muted" style={{ fontSize: 12 }}>
            {menuInstalled ? "已注册" : "未注册"}
          </span>
        </div>

        <div className="row row-wrap" style={{ marginTop: 12 }}>
          <button className="btn" onClick={toggleAutostart} disabled={busy}>
            {autostart ? "关闭开机自启" : "开启开机自启"}
          </button>
          <span className="muted" style={{ fontSize: 12 }}>
            {autostart ? "已开启" : "未开启"}
          </span>
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
