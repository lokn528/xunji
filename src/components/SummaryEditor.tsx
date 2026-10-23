import { useEffect, useState } from "react";
import { api } from "../lib/ipc";
import type { AiUsage, Card, Category } from "../lib/types";
import { errorText } from "../lib/types";
import AiUsageLine from "./AiUsageLine";
import CategoryPicker from "./CategoryPicker";
import { useToast } from "./Toast";

export default function SummaryEditor({
  card,
  categories,
  initialSuggestion,
  onCategoriesChange,
  onClose,
  onSaved,
}: {
  card: Card;
  categories: Category[];
  initialSuggestion?: {
    summary: string;
    suggestedCategories: string[];
    usage?: AiUsage | null;
  } | null;
  onCategoriesChange: (categories: Category[]) => void;
  onClose: () => void;
  onSaved: (card: Card) => void;
}) {
  const toast = useToast();
  const [summary, setSummary] = useState(card.summary);
  const [selectedCategories, setSelectedCategories] = useState<string[]>(() => {
    const names = new Map(categories.map((category) => [category.id, category.name]));
    return card.categoryIds
      .map((id) => names.get(id))
      .filter((name): name is string => Boolean(name));
  });
  const [suggestedCategories, setSuggestedCategories] = useState<string[]>([]);
  const [aiGenerated, setAiGenerated] = useState(card.aiGenerated);
  const [aiUsage, setAiUsage] = useState<AiUsage | null>(
    initialSuggestion?.usage ?? null,
  );
  const [asking, setAsking] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (initialSuggestion) {
      if (initialSuggestion.summary) {
        setSummary(initialSuggestion.summary);
        setAiGenerated(true);
      }
      setSuggestedCategories(initialSuggestion.suggestedCategories);
      setAiUsage(initialSuggestion.usage ?? null);
      setSelectedCategories((current) => [
        ...new Set([...current, ...initialSuggestion.suggestedCategories]),
      ]);
    }
  }, [initialSuggestion]);

  const askAi = async () => {
    setAsking(true);
    setAiUsage(null);
    try {
      const s = await api.analyzeCard(card.id);
      // AI 结果只填进输入框，用户点保存才落库。
      setSummary(s.summary);
      setAiGenerated(true);
      setSuggestedCategories(s.suggestedCategories);
      setAiUsage(s.usage ?? null);
      setSelectedCategories((current) => [
        ...new Set([...current, ...s.suggestedCategories]),
      ]);
      toast.ok("AI 已填入说明和分类", "确认或修改后再保存");
    } catch (e) {
      setAiUsage(null);
      toast.err("AI 不可用，可手动填写", errorText(e));
    } finally {
      setAsking(false);
    }
  };

  const save = async () => {
    setSaving(true);
    try {
      let updated = await api.setSummary(card.id, summary, aiGenerated);
      updated = await api.setCategories(card.id, selectedCategories);
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

          <div className="field">
            <span>分类</span>
            <CategoryPicker
              categories={categories}
              selected={selectedCategories}
              suggested={suggestedCategories}
              onChange={setSelectedCategories}
              onCategoriesChange={onCategoriesChange}
              disabled={saving}
            />
          </div>

          <button
            className="btn"
            onClick={askAi}
            disabled={asking}
            style={{ width: "100%" }}
          >
            {asking ? "AI 正在想…" : "让 AI 写说明"}
          </button>
          <div className="hint" style={{ marginTop: 6 }}>
            AI 会先选好建议分类；你确认、增删后再保存，也不会自动搬动文件。
          </div>
          <AiUsageLine usage={aiUsage} />
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
