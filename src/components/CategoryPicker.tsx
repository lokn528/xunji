import { useState } from "react";
import { Check, Plus, Sparkles, X } from "lucide-react";
import { api } from "../lib/ipc";
import type { Category } from "../lib/types";
import { errorText } from "../lib/types";
import { useToast } from "./Toast";

export default function CategoryPicker({
  categories,
  selected,
  suggested = [],
  onChange,
  onCategoriesChange,
  disabled = false,
}: {
  categories: Category[];
  selected: string[];
  suggested?: string[];
  onChange: (categories: string[]) => void;
  onCategoriesChange: (categories: Category[]) => void;
  disabled?: boolean;
}) {
  const toast = useToast();
  const [showCreator, setShowCreator] = useState(false);
  const [newName, setNewName] = useState("");
  const [busy, setBusy] = useState(false);

  const suggestionOnly = suggested.filter(
    (name) => !categories.some((category) => category.name === name),
  );

  const toggle = (name: string) => {
    onChange(
      selected.includes(name)
        ? selected.filter((item) => item !== name)
        : [...selected, name],
    );
  };

  const create = async () => {
    const name = newName.trim();
    if (!name || busy) return;
    setBusy(true);
    try {
      const created = await api.createCategory(name);
      const next = [...categories, created].sort((a, b) =>
        a.name.localeCompare(b.name, "zh-CN"),
      );
      onCategoriesChange(next);
      onChange([...selected, created.name]);
      setNewName("");
      setShowCreator(false);
      toast.ok("分类已建好", created.name);
    } catch (e) {
      toast.err("新建分类失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const locked = disabled || busy;

  return (
    <>
      <div className="category-chips" role="group" aria-label="选择分类">
        <button
          className={`category-chip${selected.length === 0 ? " selected" : ""}`}
          type="button"
          aria-pressed={selected.length === 0}
          onClick={() => onChange([])}
          disabled={locked}
        >
          {selected.length === 0 && <Check size={13} />}
          不分类
        </button>

        {categories.map((category) => {
          const active = selected.includes(category.name);
          const suggestedByAi = suggested.includes(category.name);
          return (
            <button
              className={`category-chip${active ? " selected" : ""}${
                suggestedByAi ? " ai-suggested" : ""
              }`}
              type="button"
              key={category.id}
              aria-pressed={active}
              onClick={() => toggle(category.name)}
              disabled={locked}
              title={
                suggestedByAi
                  ? `${category.name} · AI 建议分类`
                  : category.name
              }
            >
              {active ? (
                <Check size={13} />
              ) : suggestedByAi ? (
                <Sparkles size={13} aria-hidden="true" />
              ) : null}
              <span className="category-chip-label">{category.name}</span>
            </button>
          );
        })}

        {suggestionOnly.map((name) => {
          const active = selected.includes(name);
          return (
            <button
              className={`category-chip ai-suggested${active ? " selected" : ""}`}
              type="button"
              key={`suggested-${name}`}
              aria-pressed={active}
              onClick={() => toggle(name)}
              disabled={locked}
              title="AI 建议分类"
            >
              {active ? <Check size={13} /> : <Sparkles size={13} aria-hidden="true" />}
              <span className="category-chip-label">{name}</span>
            </button>
          );
        })}

        <button
          className="category-chip category-chip-create"
          type="button"
          onClick={() => setShowCreator((shown) => !shown)}
          disabled={locked}
          aria-expanded={showCreator}
        >
          <Plus size={13} />
          新建分类
        </button>
      </div>

      {showCreator && (
        <div className="category-creator">
          <Plus size={16} aria-hidden="true" />
          <input
            className="input"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                void create();
              }
              if (e.key === "Escape") setShowCreator(false);
            }}
            placeholder="输入新分类名称"
            autoFocus
          />
          <button
            className="btn btn-sm"
            type="button"
            onClick={() => void create()}
            disabled={!newName.trim() || busy}
          >
            <Check size={14} />
            创建
          </button>
          <button
            className="btn-icon"
            type="button"
            onClick={() => {
              setShowCreator(false);
              setNewName("");
            }}
            aria-label="取消新建分类"
            title="取消"
          >
            <X size={15} />
          </button>
        </div>
      )}
    </>
  );
}
