import { useMemo, useState } from "react";
import {
  CalendarDays,
  ChevronDown,
  ChevronUp,
  FileText,
  Plus,
  Settings2,
  Tags,
  Trash2,
} from "lucide-react";
import { api } from "../lib/ipc";
import type { Card, Category } from "../lib/types";
import { errorText } from "../lib/types";
import {
  FILE_FORMAT_OPTIONS,
  TIME_RANGE_OPTIONS,
  UNCLASSIFIED_FILTER_ID,
  countCardsByFormat,
  countCardsByTimeRange,
  filterCards,
  type CategoryMatchMode,
  type FileFormatKey,
  type TimeRange,
} from "../lib/categoryFilters";
import { useToast } from "../components/Toast";

function FilterChip({
  label,
  count,
  selected,
  onClick,
}: {
  label: string;
  count: number;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className={`category-chip${selected ? " selected" : ""}`}
      type="button"
      onClick={onClick}
      aria-pressed={selected}
      aria-label={`${label}，${count} 个文件`}
      title={label}
    >
      <span className="category-chip-label">{label}</span>
      <span className="category-chip-count">{count}</span>
    </button>
  );
}

export default function CategoryPage({
  cards,
  categories,
  onCategoriesChange,
  onCategoryDeleted,
  onOpenCard,
}: {
  cards: Card[];
  categories: Category[];
  onCategoriesChange: (categories: Category[]) => void;
  onCategoryDeleted: (categoryId: string) => void;
  onOpenCard: (card: Card) => void;
}) {
  const toast = useToast();
  const [format, setFormat] = useState<FileFormatKey | null>(null);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [matchMode, setMatchMode] = useState<CategoryMatchMode>("any");
  const [timeRange, setTimeRange] = useState<TimeRange>("all");
  const [managerOpen, setManagerOpen] = useState(false);
  const [newName, setNewName] = useState("");
  const [busy, setBusy] = useState(false);

  const categoryNames = useMemo(
    () => new Map(categories.map((category) => [category.id, category.name])),
    [categories],
  );

  const categoryCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const card of cards) {
      for (const id of card.categoryIds) {
        counts.set(id, (counts.get(id) ?? 0) + 1);
      }
    }
    return counts;
  }, [cards]);

  const formatCounts = useMemo(() => countCardsByFormat(cards), [cards]);
  const timeCounts = useMemo(() => countCardsByTimeRange(cards), [cards]);
  const unclassifiedCount = useMemo(
    () => cards.filter((card) => card.categoryIds.length === 0).length,
    [cards],
  );

  const categoryFilters = selectedIds.filter((id) => id !== UNCLASSIFIED_FILTER_ID);
  const filtered = useMemo(
    () =>
      filterCards(cards, {
        format,
        categoryIds: selectedIds,
        categoryMatchMode: matchMode,
        timeRange,
      }),
    [cards, format, matchMode, selectedIds, timeRange],
  );
  const hasActiveFilters = format !== null || selectedIds.length > 0 || timeRange !== "all";

  const create = async () => {
    const name = newName.trim();
    if (!name || busy) return;
    setBusy(true);
    try {
      const created = await api.createCategory(name);
      onCategoriesChange(
        [...categories, created].sort((a, b) => a.name.localeCompare(b.name, "zh-CN")),
      );
      setNewName("");
      toast.ok(`已创建分类「${name}」`);
    } catch (e) {
      toast.err("创建分类失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const remove = async (category: Category) => {
    if (busy) return;
    setBusy(true);
    try {
      await api.deleteCategory(category.id);
      onCategoryDeleted(category.id);
      setSelectedIds((current) => current.filter((id) => id !== category.id));
      toast.ok(`已删除分类「${category.name}」`, "文件仍在，只会移除这个标签");
    } catch (e) {
      toast.err("删除分类失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const toggleCategory = (id: string) => {
    if (id === UNCLASSIFIED_FILTER_ID) {
      setSelectedIds((current) =>
        current.includes(id) ? [] : [UNCLASSIFIED_FILTER_ID],
      );
      return;
    }
    setSelectedIds((current) => {
      const withoutUnclassified = current.filter((item) => item !== UNCLASSIFIED_FILTER_ID);
      return withoutUnclassified.includes(id)
        ? withoutUnclassified.filter((item) => item !== id)
        : [...withoutUnclassified, id];
    });
  };

  return (
    <>
      <div className="page-head">
        <h2>分类</h2>
        <p>按文件格式、内容分类和入库时间筛选全部文件。</p>
      </div>

      <section className="category-filter-panel" aria-label="文件筛选">
        <div className="category-filter-row">
          <div className="category-filter-heading">
            <div className="category-filter-title">
              <FileText size={16} aria-hidden="true" />
              <div>
                <strong id="format-filter-label">文件格式</strong>
                <span>按扩展名自动归类</span>
              </div>
            </div>
          </div>
          <div className="category-chips" role="group" aria-labelledby="format-filter-label">
            <FilterChip
              label="全部格式"
              count={cards.length}
              selected={format === null}
              onClick={() => setFormat(null)}
            />
            {FILE_FORMAT_OPTIONS.map((option) => (
              <FilterChip
                key={option.id}
                label={option.label}
                count={formatCounts[option.id]}
                selected={format === option.id}
                onClick={() => setFormat((current) => (current === option.id ? null : option.id))}
              />
            ))}
          </div>
        </div>

        <div className="category-filter-row">
          <div className="category-filter-heading">
            <div className="category-filter-title">
              <Tags size={16} aria-hidden="true" />
              <div>
                <strong id="category-filter-label">内容分类</strong>
                <span>可单选、多选或选未分类</span>
              </div>
            </div>
          </div>
          <div className="category-filter-content">
            <div className="category-filter-tools">
              {categoryFilters.length > 1 && (
                <div className="category-match-control" role="group" aria-label="多选分类匹配方式">
                  <div className="category-match-buttons">
                    <button
                      type="button"
                      className={matchMode === "any" ? "selected" : ""}
                      onClick={() => setMatchMode("any")}
                      aria-pressed={matchMode === "any"}
                    >
                      命中任一
                    </button>
                    <button
                      type="button"
                      className={matchMode === "all" ? "selected" : ""}
                      onClick={() => setMatchMode("all")}
                      aria-pressed={matchMode === "all"}
                    >
                      同时包含
                    </button>
                  </div>
                  <span>
                    {matchMode === "any"
                      ? "有其中任意一个标签即可"
                      : "必须包含已选的全部标签"}
                  </span>
                </div>
              )}
              <button
                type="button"
                className={`category-manage-trigger${managerOpen ? " selected" : ""}`}
                onClick={() => setManagerOpen((open) => !open)}
                aria-expanded={managerOpen}
                aria-controls="category-manager"
              >
                <Settings2 size={14} aria-hidden="true" />
                管理分类
                {managerOpen ? (
                  <ChevronUp size={14} aria-hidden="true" />
                ) : (
                  <ChevronDown size={14} aria-hidden="true" />
                )}
              </button>
            </div>
            <div className="category-chips" role="group" aria-labelledby="category-filter-label">
              <FilterChip
                label="不限分类"
                count={cards.length}
                selected={selectedIds.length === 0}
                onClick={() => setSelectedIds([])}
              />
              {categories.map((category) => (
                <FilterChip
                  key={category.id}
                  label={category.name}
                  count={categoryCounts.get(category.id) ?? 0}
                  selected={selectedIds.includes(category.id)}
                  onClick={() => toggleCategory(category.id)}
                />
              ))}
              <FilterChip
                label="未分类"
                count={unclassifiedCount}
                selected={selectedIds.includes(UNCLASSIFIED_FILTER_ID)}
                onClick={() => toggleCategory(UNCLASSIFIED_FILTER_ID)}
              />
            </div>
          </div>
        </div>

        <div className="category-filter-row">
          <div className="category-filter-heading">
            <div className="category-filter-title">
              <CalendarDays size={16} aria-hidden="true" />
              <div>
                <strong id="time-filter-label">时间</strong>
                <span>按文件进入循迹的时间</span>
              </div>
            </div>
          </div>
          <div className="category-chips" role="group" aria-labelledby="time-filter-label">
            {TIME_RANGE_OPTIONS.map((option) => (
              <FilterChip
                key={option.id}
                label={option.label}
                count={timeCounts[option.id]}
                selected={timeRange === option.id}
                onClick={() =>
                  setTimeRange((current) => (current === option.id ? "all" : option.id))
                }
              />
            ))}
          </div>
        </div>

        {managerOpen && (
          <div className="category-manager" id="category-manager">
            <div className="category-manager-head">
              <div>
                <strong>管理内容分类</strong>
                <span>删除分类只会移除标签，不会删除文件。</span>
              </div>
              <button
                type="button"
                className="category-manager-close"
                onClick={() => setManagerOpen(false)}
              >
                <ChevronUp size={14} aria-hidden="true" />
                收起
              </button>
            </div>

            <div className="category-manager-list">
              {categories.length === 0 ? (
                <p className="category-manager-empty">还没有自建分类。</p>
              ) : (
                categories.map((category) => (
                  <div className="category-manager-item" key={category.id}>
                    <div className="category-manager-name">
                      <strong>{category.name}</strong>
                      <span>{categoryCounts.get(category.id) ?? 0} 个文件</span>
                    </div>
                    <button
                      type="button"
                      className="category-manager-delete"
                      onClick={() => void remove(category)}
                      disabled={busy}
                      title={`删除分类「${category.name}」`}
                    >
                      <Trash2 size={14} aria-hidden="true" />
                      删除分类
                    </button>
                  </div>
                ))
              )}
            </div>

            <div className="category-manager-create">
              <input
                className="input"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void create();
                }}
                placeholder="新分类名，比如「高等数学」"
              />
              <button className="btn btn-primary" onClick={() => void create()} disabled={busy} type="button">
                <Plus size={15} aria-hidden="true" />
                新建分类
              </button>
            </div>
          </div>
        )}
      </section>

      <section className="category-results">
        <div className="category-results-head">
          <h3>{hasActiveFilters ? "筛选结果" : "全部文件"}</h3>
          <span className="badge badge-plain">{filtered.length}</span>
          {hasActiveFilters && (
            <button
              type="button"
              className="category-reset"
              onClick={() => {
                setFormat(null);
                setSelectedIds([]);
                setTimeRange("all");
                setMatchMode("any");
              }}
            >
              重置筛选
            </button>
          )}
        </div>
        {filtered.length === 0 ? (
          <div className="empty">
            <h3>没有符合条件的文件</h3>
            <p>调整文件格式、内容分类或时间筛选后再试。</p>
          </div>
        ) : (
          <div className="cat-items">
            {filtered.map((card) => (
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
                <span className="cat-item-tags">
                  {card.categoryIds.length === 0 ? (
                    <span className="badge badge-plain">未分类</span>
                  ) : (
                    card.categoryIds.map((id) => {
                      const name = categoryNames.get(id);
                      return name ? (
                        <span className="badge" key={id}>
                          {name}
                        </span>
                      ) : null;
                    })
                  )}
                </span>
              </div>
            ))}
          </div>
        )}
      </section>
    </>
  );
}
