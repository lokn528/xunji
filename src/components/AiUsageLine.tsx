import type { AiUsage } from "../lib/types";

/** 展示一次 AI 调用实际返回的 token 用量，未返回时不占位。 */
export default function AiUsageLine({
  usage,
  label = "本次补全",
}: {
  usage?: AiUsage | null;
  label?: string;
}) {
  if (!usage) return null;

  const reasoning = usage.reasoningTokens ?? 0;
  return (
    <div className="ai-usage" role="status" aria-live="polite">
      {label}：输入 {usage.inputTokens} / 输出 {usage.outputTokens} / 合计{" "}
      <strong>{usage.totalTokens}</strong> tokens
      {reasoning > 0 ? `，其中推理 ${reasoning}` : ""}
    </div>
  );
}
