// 与 Rust 侧模型一一对应的类型定义。
// 改动 Rust 结构体时必须同步这里，否则前后端会静默错位。

export type Origin = "imported_moved" | "imported_kept" | "sent_to";

export type PlacementMode = "move" | "keep";
export type AiProtocol =
  | "openai_chat"
  | "openai_responses"
  | "anthropic_messages";

export interface Card {
  id: string;
  displayName: string;
  summary: string;
  filePath: string;
  thumbPath: string | null;
  categoryIds: string[];
  origin: Origin;
  aiGenerated: boolean;
  needsSummary: boolean;
  missing: boolean;
  /** 该卡片指向的是文件夹。 */
  isDir: boolean;
  pinned: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface Category {
  id: string;
  name: string;
  createdAt: string;
  sortOrder: number;
  cardCount: number;
}

export interface Settings {
  defaultDir: string;
  placementMode: PlacementMode;
  aiProvider: string;
  aiProviderName: string;
  aiProtocol: AiProtocol;
  aiBaseUrl: string;
  aiModel: string;
  aiTemperature: number;
  aiFilenameOnly: boolean;
  thumbCacheDir: string;
  autostart: boolean;
}

export interface AiConfigInput {
  provider: string;
  providerName: string;
  protocol: AiProtocol;
  baseUrl: string;
  apiKey: string;
  model: string;
  temperature: number;
  filenameOnly: boolean;
}

export interface AiUsage {
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
  reasoningTokens?: number | null;
  cachedTokens?: number | null;
}

export interface AiSuggestion {
  summary: string;
  suggestedCategories: string[];
  raw: string;
  usage?: AiUsage | null;
}

export interface ImportRequest {
  paths: string[];
  targetDir?: string | null;
  placement?: PlacementMode | null;
  analyzeAfter: boolean;
  categories?: string[];
  note?: string | null;
  origin?: Origin | null;
}

export interface ImportItemResult {
  sourcePath: string;
  cardId?: string;
  existingCardId?: string;
  error?: string;
  errorCode?: string;
  aiSuggestion?: AiSuggestion;
}

export interface ImportResult {
  items: ImportItemResult[];
  succeeded: number;
  failed: number;
  duplicates: number;
}

export interface FolderChild {
  relativePath: string;
  size: number;
  extension: string;
  isDir: boolean;
}

export interface AppInfo {
  dataDir: string;
  dbPath: string;
  thumbDir: string;
  exePath: string;
  version: string;
}

/** Rust 侧返回的错误结构。 */
export interface AppError {
  code: string;
  message: string;
}

export function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    "code" in e &&
    "message" in e &&
    typeof (e as AppError).message === "string"
  );
}

/** 把任意异常转成可展示的中文文案。 */
export function errorText(e: unknown): string {
  if (isAppError(e)) return e.message;
  if (typeof e === "string") return e;
  return "操作失败，请重试";
}
