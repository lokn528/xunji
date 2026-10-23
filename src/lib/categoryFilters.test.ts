import { describe, expect, it } from "vitest";
import {
  UNCLASSIFIED_FILTER_ID,
  fileFormatOf,
  filterCards,
  matchesCategoryFilter,
  matchesTimeRange,
} from "./categoryFilters";
import type { Card } from "./types";

function card(overrides: Partial<Card> = {}): Card {
  return {
    id: "card-1",
    displayName: "资料.docx",
    summary: "",
    filePath: "C:\\资料\\资料.docx",
    thumbPath: null,
    categoryIds: [],
    origin: "imported_kept",
    aiGenerated: false,
    needsSummary: true,
    missing: false,
    isDir: false,
    pinned: false,
    createdAt: new Date(2026, 9, 23, 12).toISOString(),
    updatedAt: new Date(2026, 9, 23, 12).toISOString(),
    ...overrides,
  };
}

describe("fileFormatOf", () => {
  it("按扩展名识别常见格式，且不区分大小写", () => {
    expect(fileFormatOf(card({ filePath: "C:\\报告.PDF" }))).toBe("pdf");
    expect(fileFormatOf(card({ filePath: "C:\\预算.xlsx" }))).toBe("spreadsheet");
    expect(fileFormatOf(card({ filePath: "C:\\演示.pptx" }))).toBe("presentation");
    expect(fileFormatOf(card({ filePath: "C:\\备份.tar.gz" }))).toBe("archive");
  });

  it("文件夹和未知格式分别归入固定分类", () => {
    expect(fileFormatOf(card({ filePath: "C:\\项目", isDir: true }))).toBe("folder");
    expect(fileFormatOf(card({ filePath: "C:\\程序.exe" }))).toBe("other");
  });
});

describe("分类筛选", () => {
  it("匹配任一和同时包含表达不同含义", () => {
    const value = card({ categoryIds: ["study", "work"] });
    expect(matchesCategoryFilter(value, ["study", "life"], "any")).toBe(true);
    expect(matchesCategoryFilter(value, ["study", "life"], "all")).toBe(false);
    expect(matchesCategoryFilter(value, ["study", "work"], "all")).toBe(true);
  });

  it("未分类只命中没有内容标签的文件", () => {
    expect(matchesCategoryFilter(card(), [UNCLASSIFIED_FILTER_ID], "any")).toBe(true);
    expect(matchesCategoryFilter(card({ categoryIds: ["work"] }), [UNCLASSIFIED_FILTER_ID], "any")).toBe(
      false,
    );
  });
});

describe("时间筛选", () => {
  const now = new Date(2026, 9, 23, 18);

  it("支持今天、近 7 天、近 30 天和今年", () => {
    expect(matchesTimeRange(card(), "today", now)).toBe(true);
    expect(matchesTimeRange(card({ createdAt: new Date(2026, 9, 20).toISOString() }), "week", now)).toBe(
      true,
    );
    expect(
      matchesTimeRange(card({ createdAt: new Date(2026, 8, 10).toISOString() }), "month", now),
    ).toBe(false);
    expect(matchesTimeRange(card(), "year", now)).toBe(true);
  });
});

describe("filterCards", () => {
  it("格式、分类和时间三层条件同时生效", () => {
    const cards = [
      card({ id: "kept", filePath: "C:\\报告.pdf", categoryIds: ["work"] }),
      card({ id: "wrong-format", filePath: "C:\\报告.docx", categoryIds: ["work"] }),
      card({ id: "wrong-category", filePath: "C:\\报告.pdf", categoryIds: ["life"] }),
    ];

    expect(
      filterCards(
        cards,
        {
          format: "pdf",
          categoryIds: ["work"],
          categoryMatchMode: "any",
          timeRange: "today",
        },
        new Date(2026, 9, 23, 18),
      ).map((item) => item.id),
    ).toEqual(["kept"]);
  });
});
