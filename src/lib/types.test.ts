import { describe, expect, it } from "vitest";
import { errorText, isAppError } from "./types";

describe("isAppError", () => {
  it("识别 Rust 侧返回的错误结构", () => {
    expect(isAppError({ code: "DISK_FULL", message: "目标位置磁盘空间不足" })).toBe(
      true,
    );
  });

  it("不把普通对象误判为错误", () => {
    expect(isAppError({ code: 1 })).toBe(false);
    expect(isAppError({})).toBe(false);
    expect(isAppError(null)).toBe(false);
    expect(isAppError("boom")).toBe(false);
  });
});

describe("errorText", () => {
  it("优先用 Rust 侧的中文消息", () => {
    expect(errorText({ code: "AI_UNAVAILABLE", message: "AI 不可用，可手动填写" })).toBe(
      "AI 不可用，可手动填写",
    );
  });

  it("字符串错误直接返回", () => {
    expect(errorText("出错了")).toBe("出错了");
  });

  it("未知错误给出兜底文案", () => {
    expect(errorText(undefined)).toBe("操作失败，请重试");
    expect(errorText(42)).toBe("操作失败，请重试");
  });
});
