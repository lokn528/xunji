export type PageKey = "import" | "timeline" | "category" | "settings";

type PageDef = { key: PageKey; label: string; icon: string };

const PRIMARY_PAGES: PageDef[] = [
  { key: "import", label: "导入", icon: "＋" },
  { key: "timeline", label: "时间线", icon: "≣" },
  { key: "category", label: "分类", icon: "⌗" },
];

const SETTINGS_PAGE: PageDef = { key: "settings", label: "设置", icon: "⚙" };

export default function Sidebar({
  page,
  onNavigate,
  cardCount,
}: {
  page: PageKey;
  onNavigate: (p: PageKey) => void;
  cardCount: number;
}) {
  const item = (p: PageDef) => (
    <button
      key={p.key}
      onClick={() => onNavigate(p.key)}
      aria-current={page === p.key ? "page" : undefined}
    >
      <span className="ico" aria-hidden="true">
        {p.icon}
      </span>
      <span>{p.label}</span>
      {p.key === "timeline" && cardCount > 0 && <span className="spacer" />}
      {p.key === "timeline" && cardCount > 0 && (
        <span className="muted" style={{ fontSize: 11 }}>
          {cardCount}
        </span>
      )}
    </button>
  );

  return (
    <aside className="sidebar">
      {/* 左上角品牌区也是窗口拖拽区（无边框窗口没有原生标题栏可拖）。 */}
      <div className="brand" data-tauri-drag-region="deep">
        <h1>循迹</h1>
        <p>所有东西都有迹可寻</p>
      </div>

      <nav className="nav">{PRIMARY_PAGES.map(item)}</nav>

      {/* 设置单独沉到侧栏底部 */}
      <nav className="nav nav-bottom">{item(SETTINGS_PAGE)}</nav>
    </aside>
  );
}
