import { useEffect, useState } from "react";
import { api } from "../lib/ipc";
import type { Card, Category } from "../lib/types";
import { errorText } from "../lib/types";
import { useToast } from "./Toast";

export default function SummaryEditor({
  card,
  categories,
  initialSuggestion,
  onClose,
  onSaved,
}: {
  card: Card;
  categories: Category[];
  initialSuggestion?: { summary: string; category: string | null } | null;
  onClose: () => void;
  onSaved: (card: Card) => void;
}) {
  const toast = useToast();
  const [summary, setSummary] = useState(card.summary);
  const [category, setCategory] = useState("");
  const [aiGenerated, setAiGenerated] = useState(card.aiGenerated);
  const [asking, setAsking] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (initialSuggestion) {
      if (initialSuggestion.summary) {
        setSummary(initialSuggestion.summary);
        setAiGenerated(true);
      }
      if (initialSuggestion.category) {
        setCategory(initialSuggestion.category);
      }
    }
  }, [initialSuggestion]);

  const askAi = async () => {
    setAsking(true);
    try {
      const s = await api.analyzeCard(card.id);
      // AI 结果只填进输入框，用户点保存才落库。
      setSummary(s.summary);
      setAiGenerated(true);
      if (s.suggestedCategory) {
        setCategory(s.suggestedCategory);
      }
      toast.ok("AI 给了一段说明", "确认或改写后再保存");
    } catch (e) {
      toast.err("AI 不可用，可手动填写", errorText(e));
    } finally {
      setAsking(false);
    }
  };

  const save = async () => {
    setSaving(true);
    try {
      let updated = await api.setSummary(card.id, summary, aiGenerated);
      if (category.trim()) {
        updated = await api.setCategory(card.id, category.trim());
      }
      onSaved(updated);
      toast.ok("已保存");
      onClose();
    } catch (e) {
      toast.err("保存失败", errorText(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <header>
          <h3>说明</h3>
          <button className="btn-icon" onClick={onClose} aria-label="关闭">
            ✕
          </button>
        </header>

        <div className="body">
          <div className="muted mono" style={{ fontSize: 11, marginBottom: 10 }}>
            {card.filePath}
          </div>

          <label className="field">
            <span>这是一份什么东西？</span>
            <textarea
              className="textarea"
              value={summary}
              onChange={(e) => {
                setSummary(e.target.value);
                // 手改过就不再标记为 AI 生成
                if (aiGenerated) setAiGenerated(false);
              }}
              placeholder="一句话说清用途，比如「大二下线性代数教材，第三章作业要用」"
            />
            <span className="hint">
              {summary.length}/50 字。留空则这张卡片标记为「待写」。
            </span>
          </label>

          <label className="field">
            <span>分类</span>
            <input
              className="input"
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              placeholder="留空表示不分类"
              list="category-options"
            />
            <datalist id="category-options">
              {categories.map((c) => (
                <option key={c.id} value={c.name} />
              ))}
            </datalist>
            <span className="hint">
              填一个不存在的名字会自动创建该分类。
            </span>
          </label>

          <button
            className="btn"
            onClick={askAi}
            disabled={asking}
            style={{ width: "100%" }}
          >
            {asking ? "AI 正在想…" : "让 AI 写说明"}
          </button>
          <div className="hint" style={{ marginTop: 6 }}>
            AI 只给建议，不会自动保存，也不会自动搬动你的文件。
          </div>
        </div>

        <footer>
          <button className="btn" onClick={onClose}>
            取消
          </button>
          <button className="btn btn-primary" onClick={save} disabled={saving}>
            {saving ? "保存中…" : "保存"}
          </button>
        </footer>
      </div>
    </div>
  );
}
