import { useMemo } from "react";
import CardTile from "../components/CardTile";
import { dateKey, formatDate } from "../lib/ipc";
import type { Card, Category } from "../lib/types";

export default function TimelinePage({
  cards,
  categories,
  onOpenSummary,
  onChanged,
  onRelink,
}: {
  cards: Card[];
  categories: Category[];
  onOpenSummary: (card: Card) => void;
  onChanged: (card: Card) => void;
  onRelink: (card: Card) => void;
}) {
  const namesOf = useMemo(() => {
    const map = new Map(categories.map((c) => [c.id, c.name]));
    return (ids: string[]) =>
      ids.map((id) => map.get(id)).filter((name): name is string => Boolean(name));
  }, [categories]);

  const pinned = cards.filter((c) => c.pinned);
  const rest = cards.filter((c) => !c.pinned);

  // 按日期分组，组内倒序（Rust 侧已按 created_at DESC 排好）。
  const groups = useMemo(() => {
    const map = new Map<string, Card[]>();
    for (const card of rest) {
      const key = dateKey(card.createdAt);
      const list = map.get(key);
      if (list) list.push(card);
      else map.set(key, [card]);
    }
    return [...map.entries()];
  }, [rest]);

  if (cards.length === 0) {
    return (
      <>
        <div className="page-head">
          <h2>时间线</h2>
          <p>按导入日期排的卡片流。</p>
        </div>
        <div className="empty">
          <h3>还没有卡片</h3>
          <p>去「导入」把文件丢进来，这里就会按时间排开。</p>
        </div>
      </>
    );
  }

  return (
    <>
      <div className="page-head">
        <h2>时间线</h2>
        <p>
          共 {cards.length} 张卡片。点卡片上的「位置」能在资源管理器里找到它。
        </p>
      </div>

      {pinned.length > 0 && (
        <div className="pinned-strip">
          <h3>近期要用 · {pinned.length}</h3>
          <div className="card-grid">
            {pinned.map((card) => (
              <CardTile
                key={card.id}
                card={card}
                categoryNames={namesOf(card.categoryIds)}
                onOpenSummary={onOpenSummary}
                onChanged={onChanged}
                onRelink={onRelink}
              />
            ))}
          </div>
        </div>
      )}

      {groups.map(([key, items]) => (
        <section className="day-group" key={key}>
          <div className="day-head">
            <h3>{formatDate(items[0].createdAt)}</h3>
            <span className="count">{items.length} 张</span>
          </div>
          <div className="card-grid">
            {items.map((card) => (
              <CardTile
                key={card.id}
                card={card}
                categoryNames={namesOf(card.categoryIds)}
                onOpenSummary={onOpenSummary}
                onChanged={onChanged}
                onRelink={onRelink}
              />
            ))}
          </div>
        </section>
      ))}
    </>
  );
}
