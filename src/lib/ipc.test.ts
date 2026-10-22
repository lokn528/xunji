import { describe, expect, it } from "vitest";
import { dateKey, formatDate, formatSize, formatTime } from "./ipc";

describe("formatSize", () => {
  it("小于 1KB 显示原始字节数", () => {
    expect(formatSize(0)).toBe("0 B");
    expect(formatSize(512)).toBe("512 B");
  });

  it("按量级换算", () => {
    expect(formatSize(1024)).toBe("1.0 KB");
    expect(formatSize(1024 * 1024)).toBe("1.0 MB");
    expect(formatSize(1024 * 1024 * 1024)).toBe("1.0 GB");
  });

  it("超过 10 的单位取整到一位小数", () => {
    expect(formatSize(15 * 1024)).toBe("15 KB");
  });
});

describe("dateKey", () => {
  it("取出 ISO 时间戳的日期部分", () => {
    expect(dateKey("2026-10-22T08:30:00.000Z")).toBe("2026-10-22");
  });

  it("同一时刻的分组键稳定", () => {
    const a = dateKey("2026-10-22T00:00:00.000Z");
    const b = dateKey("2026-10-22T23:59:59.999Z");
    expect(a).toBe(b);
  });
});

describe("formatDate / formatTime", () => {
  it("把日期格式化成中文", () => {
    expect(formatDate("2026-10-22T08:30:00.000Z")).toContain("2026");
    expect(formatDate("2026-10-22T08:30:00.000Z")).toContain("月");
  });

  it("时间补零", () => {
    const d = new Date(2026, 9, 22, 9, 5, 0);
    expect(formatTime(d.toISOString())).toBe("09:05");
  });

  it("非法时间戳不抛异常", () => {
    expect(formatDate("not-a-date")).toBe("not-a-date");
    expect(formatTime("not-a-date")).toBe("");
  });
});
