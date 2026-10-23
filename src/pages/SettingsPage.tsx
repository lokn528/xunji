import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Cable,
  ChevronDown,
  Eye,
  EyeOff,
  KeyRound,
  LoaderCircle,
  MousePointer2,
  Power,
  Server,
  Sparkles,
  Trash2,
} from "lucide-react";
import { api } from "../lib/ipc";
import type { AiProtocol, Settings } from "../lib/types";
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

interface ProviderPreset {
  provider: string;
  name: string;
  label: string;
  baseUrl: string;
  protocol: AiProtocol;
  model: string;
  models: readonly string[];
}

const PROVIDER_PRESETS: readonly ProviderPreset[] = [
  {
    provider: "openai",
    name: "OpenAI",
    label: "OpenAI 官方",
    baseUrl: "https://api.openai.com/v1",
    protocol: "openai_chat",
    model: "gpt-4o-mini",
    models: ["gpt-4o-mini", "gpt-4.1-mini", "gpt-4.1"],
  },
  {
    provider: "anthropic",
    name: "Anthropic",
    label: "Anthropic",
    baseUrl: "https://api.anthropic.com/v1",
    protocol: "anthropic_messages",
    model: "claude-sonnet-4-5",
    models: ["claude-sonnet-4-5", "claude-haiku-4-5", "claude-opus-4-1"],
  },
  {
    provider: "deepseek",
    name: "DeepSeek",
    label: "DeepSeek",
    baseUrl: "https://api.deepseek.com/v1",
    protocol: "openai_chat",
    model: "deepseek-chat",
    models: ["deepseek-chat", "deepseek-reasoner"],
  },
  {
    provider: "moonshot",
    name: "Moonshot AI",
    label: "Kimi / Moonshot",
    baseUrl: "https://api.moonshot.cn/v1",
    protocol: "openai_chat",
    model: "moonshot-v1-8k",
    models: ["moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k"],
  },
  {
    provider: "zhipu",
    name: "智谱 AI",
    label: "智谱 GLM",
    baseUrl: "https://open.bigmodel.cn/api/paas/v4",
    protocol: "openai_chat",
    model: "glm-4-flash",
    models: ["glm-4-flash", "glm-4-plus", "glm-4-air"],
  },
  {
    provider: "qwen",
    name: "通义千问",
    label: "通义千问",
    baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    protocol: "openai_chat",
    model: "qwen-plus",
    models: ["qwen-plus", "qwen-turbo", "qwen-max"],
  },
];

const PROTOCOLS: readonly {
  value: AiProtocol;
  label: string;
  endpoint: string;
  description: string;
  models: readonly string[];
}[] = [
  {
    value: "openai_chat",
    label: "OpenAI Chat Completions",
    endpoint: "/chat/completions",
    description: "适用于 OpenAI 兼容接口、多数中转站和国产模型。",
    models: ["gpt-4o-mini", "deepseek-chat", "glm-4-flash", "qwen-plus"],
  },
  {
    value: "openai_responses",
    label: "OpenAI Responses",
    endpoint: "/responses",
    description: "使用 OpenAI 新版 Responses 请求与响应格式。",
    models: ["gpt-4.1-mini", "gpt-4.1", "o4-mini"],
  },
  {
    value: "anthropic_messages",
    label: "Anthropic Messages",
    endpoint: "/messages",
    description: "使用 Anthropic 原生 Messages 格式与 x-api-key 鉴权。",
    models: ["claude-sonnet-4-5", "claude-haiku-4-5", "claude-opus-4-1"],
  },
];

type ConnectionMode = "provider" | "custom";
type AiField = "provider" | "providerName" | "baseUrl" | "model" | "apiKey";
type FieldErrors = Partial<Record<AiField, string>>;

function findPreset(provider: string): ProviderPreset | undefined {
  return PROVIDER_PRESETS.find((preset) => preset.provider === provider);
}

function validProviderId(value: string): boolean {
  return /^[a-z][a-z0-9_-]*$/.test(value.trim());
}

function validBaseUrl(value: string): boolean {
  try {
    const url = new URL(value.trim());
    return url.protocol === "https:" || url.protocol === "http:";
  } catch {
    return false;
  }
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
  const [mode, setMode] = useState<ConnectionMode>("custom");
  const [presetProvider, setPresetProvider] = useState("openai");
  const [provider, setProvider] = useState("openai_compatible");
  const [providerName, setProviderName] = useState("OpenAI 兼容");
  const [protocol, setProtocol] = useState<AiProtocol>("openai_chat");
  const [baseUrl, setBaseUrl] = useState("");
  const [model, setModel] = useState("");
  const [temperature, setTemperature] = useState(0.3);
  const [filenameOnly, setFilenameOnly] = useState(true);
  const [apiKey, setApiKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [keyPresent, setKeyPresent] = useState(false);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [menuInstalled, setMenuInstalled] = useState(false);
  const [autostart, setAutostart] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!settings) return;
    setDefaultDir(settings.defaultDir);
    setPlacementMode(settings.placementMode);
    setProvider(settings.aiProvider);
    setProtocol(settings.aiProtocol);
    setBaseUrl(settings.aiBaseUrl);
    setModel(settings.aiModel);
    setTemperature(settings.aiTemperature);
    setFilenameOnly(settings.aiFilenameOnly);

    const preset = findPreset(settings.aiProvider);
    setProviderName(
      preset && settings.aiProviderName === "OpenAI 兼容"
        ? preset.name
        : settings.aiProviderName,
    );
    setMode(preset ? "provider" : "custom");
    setPresetProvider(preset?.provider ?? PROVIDER_PRESETS[0].provider);
  }, [settings]);

  useEffect(() => {
    api.hasApiKey().then(setKeyPresent).catch(() => setKeyPresent(false));
    api.getAppInfo().then(setInfo).catch(() => undefined);
    api.shellMenuInstalled?.().then(setMenuInstalled).catch(() => undefined);
    api.getAutostart().then(setAutostart).catch(() => setAutostart(false));
  }, []);

  const selectedProtocol = useMemo(
    () => PROTOCOLS.find((item) => item.value === protocol) ?? PROTOCOLS[0],
    [protocol],
  );

  const modelSuggestions = useMemo(() => {
    if (mode === "provider") {
      return findPreset(presetProvider)?.models ?? selectedProtocol.models;
    }
    return selectedProtocol.models;
  }, [mode, presetProvider, selectedProtocol]);

  const providerChanged = Boolean(settings && provider !== settings.aiProvider);

  const applyPreset = (preset: ProviderPreset) => {
    setPresetProvider(preset.provider);
    setProvider(preset.provider);
    setProviderName(preset.name);
    setProtocol(preset.protocol);
    setBaseUrl(preset.baseUrl);
    setModel(preset.model);
    setApiKey("");
    setShowKey(false);
    setFieldErrors({});
  };

  const changeMode = (nextMode: ConnectionMode) => {
    if (nextMode === mode) return;
    setMode(nextMode);
    setFieldErrors({});
    setApiKey("");
    setShowKey(false);

    if (nextMode === "provider") {
      const preset =
        findPreset(provider) ??
        findPreset(presetProvider) ??
        PROVIDER_PRESETS[0];
      applyPreset(preset);
      return;
    }

    if (!validProviderId(provider)) setProvider("custom-gateway");
    if (!providerName.trim()) setProviderName("自定义 API");
  };

  const chooseDir = async () => {
    const picked = await open({
      multiple: false,
      directory: true,
      title: "选择默认存储位置",
    });
    if (!picked || Array.isArray(picked)) return;
    setDefaultDir(picked);
  };

  const errorFor = (field: AiField): string => {
    switch (field) {
      case "provider":
        return validProviderId(provider)
          ? ""
          : "以小写字母开头，仅使用小写字母、数字、连字符或下划线";
      case "providerName":
        return providerName.trim() ? "" : "请填写供应商显示名称";
      case "baseUrl":
        return validBaseUrl(baseUrl)
          ? ""
          : "请填写以 http:// 或 https:// 开头的 API 地址";
      case "model":
        return model.trim() ? "" : "请填写模型名称";
      case "apiKey":
        if (apiKey.trim()) return "";
        if (!keyPresent) return "请填写 API Key";
        if (providerChanged) return "更换提供商后，请重新填写这个提供商的 API Key";
        return "";
    }
  };

  const validateField = (field: AiField) => {
    const message = errorFor(field);
    setFieldErrors((current) => ({
      ...current,
      [field]: message || undefined,
    }));
    return !message;
  };

  const validateAi = (): boolean => {
    const next: FieldErrors = {};
    const providerMessage =
      mode === "custom" ? errorFor("provider") : "";
    if (providerMessage) next.provider = providerMessage;
    if (!providerName.trim()) next.providerName = "请填写供应商显示名称";
    if (!validBaseUrl(baseUrl)) {
      next.baseUrl = "请填写以 http:// 或 https:// 开头的 API 地址";
    }
    if (!model.trim()) next.model = "请填写模型名称";
    if (!apiKey.trim()) {
      if (!keyPresent) {
        next.apiKey = "请填写 API Key";
      } else if (providerChanged) {
        next.apiKey = "更换提供商后，请重新填写这个提供商的 API Key";
      }
    }
    setFieldErrors(next);
    return Object.keys(next).length === 0;
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

  const saveAi = async (testAfter: boolean) => {
    if (!validateAi()) return;
    setBusy(true);
    try {
      const next = await api.setAiConfig({
        provider,
        providerName,
        protocol,
        baseUrl,
        apiKey,
        model,
        temperature,
        filenameOnly,
      });
      onSettingsChange(next);
      setApiKey("");
      setShowKey(false);
      setKeyPresent(await api.hasApiKey());
      toast.ok(testAfter ? "配置已保存，正在检查连接" : "模型接入已保存");

      if (testAfter) {
        try {
          const message = await api.testAiConnection();
          toast.ok("连接正常", message);
        } catch (e) {
          toast.err("配置已保存，但连接失败", errorText(e));
        }
      }
    } catch (e) {
      toast.err("保存失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const clearKey = async () => {
    setBusy(true);
    try {
      await api.clearApiKey();
      setKeyPresent(false);
      setApiKey("");
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
      const message = menuInstalled
        ? await api.uninstallShellMenu()
        : await api.installShellMenu();
      setMenuInstalled(!menuInstalled);
      toast.ok(message);
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
        <p>文件留在本机，模型密钥只存进系统凭据库。</p>
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
            <button className="btn" onClick={chooseDir} type="button">
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

      <section className="panel ai-settings-panel">
        <div className="section-title-row">
          <h3>模型接入</h3>
          <span className={`badge ${keyPresent ? "badge-ok" : "badge-warn"}`}>
            {keyPresent ? "密钥已保存" : "尚未配置密钥"}
          </span>
        </div>

        <div className="settings-tabs" role="tablist" aria-label="模型接入方式">
          <button
            className={`settings-tab${mode === "provider" ? " is-active" : ""}`}
            type="button"
            role="tab"
            aria-selected={mode === "provider"}
            onClick={() => changeMode("provider")}
          >
            <Server size={17} aria-hidden="true" />
            <span>
              <strong>第三方模型提供商</strong>
              <small>从常用服务中选择</small>
            </span>
          </button>
          <button
            className={`settings-tab${mode === "custom" ? " is-active" : ""}`}
            type="button"
            role="tab"
            aria-selected={mode === "custom"}
            onClick={() => changeMode("custom")}
          >
            <Cable size={17} aria-hidden="true" />
            <span>
              <strong>自定义模型 API</strong>
              <small>中转站或自建网关</small>
            </span>
          </button>
        </div>

        <div className="connection-summary">
          <div className="protocol-icon" aria-hidden="true">
            <Cable size={17} />
          </div>
          <div>
            <strong>
              {mode === "provider" ? providerName : providerName || "自定义 API"}
            </strong>
            <span>
              {selectedProtocol.label} · {selectedProtocol.endpoint}
            </span>
          </div>
        </div>

        {mode === "provider" ? (
          <>
            <label className="field">
              <span>提供商</span>
              <select
                className="select"
                value={presetProvider}
                onChange={(e) => {
                  const preset = findPreset(e.target.value);
                  if (preset) applyPreset(preset);
                }}
              >
                {PROVIDER_PRESETS.map((preset) => (
                  <option key={preset.provider} value={preset.provider}>
                    {preset.label}
                  </option>
                ))}
              </select>
              <span className="hint">
                会自动填入该服务的 API 地址、协议和常用模型，仍可在高级设置中修改。
              </span>
            </label>
          </>
        ) : (
          <div className="ai-form-grid">
            <label className="field">
              <span>Provider ID</span>
              <input
                className="input mono"
                value={provider}
                onChange={(e) => {
                  setProvider(e.target.value);
                  setFieldErrors((current) => ({ ...current, provider: undefined }));
                }}
                onBlur={() => validateField("provider")}
                placeholder="acme-gateway"
                aria-invalid={Boolean(fieldErrors.provider)}
              />
              {fieldErrors.provider ? (
                <span className="field-error" role="alert">
                  {fieldErrors.provider}
                </span>
              ) : (
                <span className="hint">用于区分接入配置，以小写字母开头。</span>
              )}
            </label>

            <label className="field">
              <span>显示名称</span>
              <input
                className="input"
                value={providerName}
                onChange={(e) => {
                  setProviderName(e.target.value);
                  setFieldErrors((current) => ({ ...current, providerName: undefined }));
                }}
                onBlur={() => validateField("providerName")}
                placeholder="公司模型网关"
                aria-invalid={Boolean(fieldErrors.providerName)}
              />
              {fieldErrors.providerName ? (
                <span className="field-error" role="alert">
                  {fieldErrors.providerName}
                </span>
              ) : (
                <span className="hint">设置页内显示的名称。</span>
              )}
            </label>
          </div>
        )}

        <div className={mode === "custom" ? "ai-form-grid" : "ai-form-grid single"}>
          <label className="field">
            <span>API 地址</span>
            <input
              className="input mono"
              value={baseUrl}
              onChange={(e) => {
                setBaseUrl(e.target.value);
                setFieldErrors((current) => ({ ...current, baseUrl: undefined }));
              }}
              onBlur={() => validateField("baseUrl")}
              placeholder="https://gateway.example/v1"
              aria-invalid={Boolean(fieldErrors.baseUrl)}
            />
            {fieldErrors.baseUrl ? (
              <span className="field-error" role="alert">
                {fieldErrors.baseUrl}
              </span>
            ) : (
              <span className="hint">
                填到版本路径即可，例如 /v1；请求路径会自动补全。
              </span>
            )}
          </label>

          {mode === "custom" && (
            <label className="field">
              <span>API 协议</span>
              <select
                className="select"
                value={protocol}
                onChange={(e) => setProtocol(e.target.value as AiProtocol)}
              >
                {PROTOCOLS.map((item) => (
                  <option key={item.value} value={item.value}>
                    {item.label}
                  </option>
                ))}
              </select>
              <span className="hint">{selectedProtocol.description}</span>
            </label>
          )}
        </div>

        <label className="field">
          <span className="field-label-row">
            <span>API Key</span>
            {keyPresent && <span className="key-saved">已保存</span>}
          </span>
          <div className="input-action-row">
            <KeyRound className="input-leading-icon" size={16} aria-hidden="true" />
            <input
              className="input mono has-leading-icon"
              type={showKey ? "text" : "password"}
              value={apiKey}
              onChange={(e) => {
                setApiKey(e.target.value);
                setFieldErrors((current) => ({ ...current, apiKey: undefined }));
              }}
              onBlur={() => validateField("apiKey")}
              placeholder={
                keyPresent
                  ? providerChanged
                    ? "更换了提供商，请填写新密钥"
                    : "已保存，留空表示不改动"
                  : "输入 API Key"
              }
              autoComplete="off"
              aria-invalid={Boolean(fieldErrors.apiKey)}
            />
            <button
              className="btn-icon key-action"
              type="button"
              onClick={() => setShowKey((shown) => !shown)}
              aria-label={showKey ? "隐藏 API Key" : "显示 API Key"}
              title={showKey ? "隐藏 API Key" : "显示 API Key"}
            >
              {showKey ? <EyeOff size={15} /> : <Eye size={15} />}
            </button>
            {keyPresent && (
              <button
                className="btn-icon key-action"
                type="button"
                onClick={() => void clearKey()}
                disabled={busy}
                aria-label="清除 API Key"
                title="清除 API Key"
              >
                <Trash2 size={15} />
              </button>
            )}
          </div>
          {fieldErrors.apiKey ? (
            <span className="field-error" role="alert">
              {fieldErrors.apiKey}
            </span>
          ) : (
            <span className="hint">密钥存入 Windows 凭据库，不写入数据库或配置文件。</span>
          )}
        </label>

        <label className="field">
          <span>模型</span>
          <input
            className="input mono"
            value={model}
            onChange={(e) => {
              setModel(e.target.value);
              setFieldErrors((current) => ({ ...current, model: undefined }));
            }}
            onBlur={() => validateField("model")}
            placeholder="输入模型 ID"
            list="ai-model-suggestions"
            aria-invalid={Boolean(fieldErrors.model)}
          />
          <datalist id="ai-model-suggestions">
            {modelSuggestions.map((item) => (
              <option key={item} value={item} />
            ))}
          </datalist>
          {fieldErrors.model ? (
            <span className="field-error" role="alert">
              {fieldErrors.model}
            </span>
          ) : (
            <span className="hint">
              可从建议中选择，也可以直接填写服务商提供的模型 ID。
            </span>
          )}
        </label>

        <details className="ai-advanced">
          <summary>
            <ChevronDown size={15} aria-hidden="true" />
            高级设置
          </summary>
          <div className="ai-advanced-body">
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
                越低越稳定，越高越发散；资料整理建议保持 0.2～0.4。
              </span>
            </label>

            <div className="setting-toggle-row">
              <div className="setting-toggle-copy">
                <div className="setting-icon" aria-hidden="true">
                  <Sparkles size={16} />
                </div>
                <div>
                  <strong>仅用文件名分析</strong>
                  <span>只发送文件名与类型；关闭后文本文件会附上开头一段。</span>
                </div>
              </div>
              <Switch
                checked={filenameOnly}
                onChange={setFilenameOnly}
                label="仅用文件名分析"
              />
            </div>
          </div>
        </details>

        <div className="ai-actions">
          <button
            className="btn btn-primary"
            onClick={() => void saveAi(true)}
            disabled={busy}
          >
            {busy ? (
              <LoaderCircle className="spin" size={15} aria-hidden="true" />
            ) : (
              <Cable size={15} aria-hidden="true" />
            )}
            {busy ? "正在检查…" : "保存并测试"}
          </button>
          <button className="btn" onClick={() => void saveAi(false)} disabled={busy}>
            仅保存
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
