import { useMemo, useState } from "react";
import { api } from "../lib/ipc";
import type { Card, Category } from "../lib/types";
import { errorText } from "../lib/types";
import { useToast } from "../components/Toast";

export default function CategoryPage({
  cards,
  categories,
  onChanged,
  onOpenCard,
}: {
  cards: Card[];
  categories: Category[];
  onChanged: (card: Card) => void;
  onOpenCard: (card: Card) => void;
}) {
  const toast = useToast();
  const [filter, setFilter] = useState<string | null>(null);
  const [newName, setNewName] = useState("");
  const [busy, setBusy] = useState(false);

  const grouped = useMemo(() => {
    const map = new Map<string, Card[]>();
    for (const c of cards) {
      const key = c.categoryId ?? "__none__";
      const list = map.get(key);
      if (list) list.push(c);
      else map.set(key, [c]);
    }
    return map;
  }, [cards]);

  const create = async () => {
    const name = newName.trim();
    if (!name) return;
    setBusy(true);
    try {
      await api.createCategory(name);
      setNewName("");
      toast.ok(`已创建分类「${name}」`);
    } catch (e) {
      toast.err("创建分类失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const remove = async (cat: Category) => {
    setBusy(true);
    try {
      await api.deleteCategory(cat.id);
      toast.ok(`已删除分类「${cat.name}」`, "下面的卡片还在，只是变成未分类");
    } catch (e) {
      toast.err("删除分类失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const assign = async (card: Card, name: string) => {
    try {
      const updated = await api.setCategory(card.id, name);
      onChanged(updated);
    } catch (e) {
      toast.err("归类失败", errorText(e));
    }
  };

  const visible = filter
    ? categories.filter((c) => c.id === filter)
    : categories;
  const unclassified = grouped.get("__none__") ?? [];

  return (
    <>
      <div className="page-head">
        <h2>分类</h2>
        <p>按分类归拢卡片。卡片归到不存在的分类时会自动创建它。</p>
      </div>

      <div className="row row-wrap" style={{ marginBottom: 16 }}>
        <input
          className="input"
          style={{ maxWidth: 240 }}
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void create();
          }}
          placeholder="新分类名，比如「高等数学」"
        />
        <button className="btn btn-primary" onClick={create} disabled={busy}>
          创建分类
        </button>
        <span className="spacer" />
        {filter && (
          <button className="btn btn-sm" onClick={() => setFilter(null)}>
            取消过滤
          </button>
        )}
      </div>

      {categories.length === 0 && unclassified.length === 0 ? (
        <div className="empty">
          <h3>还没有分类</h3>
          <p>先建一个，比如「课程作业」「毕设」「文献」。</p>
        </div>
      ) : (
        <div className="cat-grid">
          {visible.map((cat) => {
            const items = grouped.get(cat.id) ?? [];
            return (
              <section className="cat-block" key={cat.id}>
                <div className="cat-head">
                  <h3>{cat.name}</h3>
                  <span className="badge badge-plain">{items.length}</span>
                  <button
                    className="btn-icon"
                    onClick={() => void remove(cat)}
                    disabled={busy}
                    title="删除分类（卡片保留）"
                  >
                    ✕
                  </button>
                </div>
                <div className="cat-items">
                  {items.length === 0 ? (
                    <div
                      className="muted"
                      style={{ fontSize: 12, padding: "6px 0" }}
                    >
                      还没有卡片
                    </div>
                  ) : (
                    items.map((card) => (
                      <div
                        className="cat-item"
                        key={card.id}
                        onClick={() => onOpenCard(card)}
                        title={card.filePath}
                      >
                        <span className="name">{card.displayName}</span>
                        <span className="sum">
                          {card.summary || (card.missing ? "源文件丢失" : "待写")}
                        </span>
                      </div>
                    ))
                  )}
                </div>
              </section>
            );
          })}

          {unclassified.length > 0 && (
            <section className="cat-block">
              <div className="cat-head">
                <h3>未分类</h3>
                <span className="badge badge-plain">{unclassified.length}</span>
              </div>
              <div className="cat-items">
                {unclassified.map((card) => (
                  <div
                    className="cat-item"
                    key={card.id}
                    onClick={() => onOpenCard(card)}
                    title={card.filePath}
                  >
                    <span className="name">{card.displayName}</span>
                    <select
                      className="select"
                      style={{ width: 120, padding: "3px 6px", fontSize: 12 }}
                      value=""
                      onClick={(e) => e.stopPropagation()}
                      onChange={(e) => {
                        if (e.target.value) void assign(card, e.target.value);
                      }}
                    >
                      <option value="">归类到…</option>
                      {categories.map((c) => (
                        <option key={c.id} value={c.name}>
                          {c.name}
                        </option>
                      ))}
                    </select>
                  </div>
                ))}
              </div>
            </section>
          )}
        </div>
      )}
    </>
  );
}
