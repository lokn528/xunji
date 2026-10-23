import type { Card } from "./types";

export const UNCLASSIFIED_FILTER_ID = "__none__";

export const FILE_FORMAT_OPTIONS = [
  { id: "document", label: "文档" },
  { id: "spreadsheet", label: "表格" },
  { id: "presentation", label: "演示" },
  { id: "pdf", label: "PDF" },
  { id: "text", label: "文本" },
  { id: "image", label: "图片" },
  { id: "media", label: "音视频" },
  { id: "archive", label: "压缩包" },
  { id: "folder", label: "文件夹" },
  { id: "other", label: "其他" },
] as const;

export type FileFormatKey = (typeof FILE_FORMAT_OPTIONS)[number]["id"];

export const TIME_RANGE_OPTIONS = [
  { id: "all", label: "全部时间" },
  { id: "today", label: "今天" },
  { id: "week", label: "近 7 天" },
  { id: "month", label: "近 30 天" },
  { id: "year", label: "今年" },
] as const;

export type TimeRange = (typeof TIME_RANGE_OPTIONS)[number]["id"];
export type CategoryMatchMode = "any" | "all";

type FilterableCard = Pick<
  Card,
  "categoryIds" | "createdAt" | "displayName" | "filePath" | "isDir"
>;

const FORMAT_EXTENSIONS: Array<[Exclude<FileFormatKey, "folder" | "other">, readonly string[]]> = [
  ["document", ["doc", "docx", "docm", "dot", "dotx", "odt", "rtf", "pages", "wps"]],
  ["spreadsheet", ["xls", "xlsx", "xlsm", "xlsb", "csv", "tsv", "ods", "numbers", "et"]],
  ["presentation", ["ppt", "pptx", "pptm", "odp", "key", "dps"]],
  ["pdf", ["pdf"]],
  ["text", ["txt", "md", "markdown", "log", "json", "xml", "yaml", "yml"]],
  ["image", ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "tif", "tiff", "heic", "avif", "ico", "psd"]],
  ["media", ["mp3", "wav", "flac", "aac", "m4a", "ogg", "opus", "wma", "mp4", "mov", "avi", "mkv", "webm", "wmv", "m4v", "flv", "mpeg", "mpg"]],
  ["archive", ["zip", "rar", "7z", "tar", "gz", "tgz", "bz2", "xz", "zst"]],
];

function extensionOf(path: string): string {
  const fileName = path.split(/[\\/]/).pop() ?? "";
  const dot = fileName.lastIndexOf(".");
  return dot > 0 && dot < fileName.length - 1 ? fileName.slice(dot + 1).toLowerCase() : "";
}

export function fileFormatOf(card: Pick<FilterableCard, "displayName" | "filePath" | "isDir">): FileFormatKey {
  if (card.isDir) return "folder";

  const extension = extensionOf(card.filePath || card.displayName);
  for (const [format, extensions] of FORMAT_EXTENSIONS) {
    if (extensions.includes(extension)) return format;
  }
  return "other";
}

export function countCardsByFormat(cards: readonly FilterableCard[]): Record<FileFormatKey, number> {
  const counts = Object.fromEntries(FILE_FORMAT_OPTIONS.map((option) => [option.id, 0])) as Record<
    FileFormatKey,
    number
  >;
  for (const card of cards) counts[fileFormatOf(card)] += 1;
  return counts;
}

export function countCardsByTimeRange(
  cards: readonly FilterableCard[],
  now = new Date(),
): Record<TimeRange, number> {
  const counts = Object.fromEntries(TIME_RANGE_OPTIONS.map((option) => [option.id, 0])) as Record<
    TimeRange,
    number
  >;
  counts.all = cards.length;
  for (const card of cards) {
    for (const option of TIME_RANGE_OPTIONS) {
      if (option.id !== "all" && matchesTimeRange(card, option.id, now)) counts[option.id] += 1;
    }
  }
  return counts;
}

function dateOf(card: Pick<FilterableCard, "createdAt">): Date | null {
  const date = new Date(card.createdAt);
  return Number.isNaN(date.getTime()) ? null : date;
}

function startOfDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

export function matchesTimeRange(
  card: Pick<FilterableCard, "createdAt">,
  range: TimeRange,
  now = new Date(),
): boolean {
  if (range === "all") return true;
  const createdAt = dateOf(card);
  if (!createdAt) return false;

  if (range === "today") {
    const start = startOfDay(now);
    const end = new Date(start);
    end.setDate(end.getDate() + 1);
    return createdAt >= start && createdAt < end;
  }

  if (range === "year") return createdAt.getFullYear() === now.getFullYear();

  const start = startOfDay(now);
  start.setDate(start.getDate() - (range === "week" ? 7 : 30));
  return createdAt >= start;
}

export function matchesCategoryFilter(
  card: Pick<FilterableCard, "categoryIds">,
  selectedIds: readonly string[],
  matchMode: CategoryMatchMode,
): boolean {
  if (selectedIds.length === 0) return true;
  if (selectedIds.includes(UNCLASSIFIED_FILTER_ID)) return card.categoryIds.length === 0;
  if (matchMode === "all") return selectedIds.every((id) => card.categoryIds.includes(id));
  return selectedIds.some((id) => card.categoryIds.includes(id));
}

export interface CardFilters {
  format: FileFormatKey | null;
  categoryIds: readonly string[];
  categoryMatchMode: CategoryMatchMode;
  timeRange: TimeRange;
}

export function filterCards<T extends FilterableCard>(
  cards: readonly T[],
  filters: CardFilters,
  now = new Date(),
): T[] {
  return cards.filter((card) => {
    if (filters.format && fileFormatOf(card) !== filters.format) return false;
    if (!matchesCategoryFilter(card, filters.categoryIds, filters.categoryMatchMode)) return false;
    return matchesTimeRange(card, filters.timeRange, now);
  });
}
