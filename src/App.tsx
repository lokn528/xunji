import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import Sidebar from "./components/Sidebar";
import type { PageKey } from "./components/Sidebar";
import TitleBar from "./components/TitleBar";
import SummaryEditor from "./components/SummaryEditor";
import { ToastProvider, useToast } from "./components/Toast";
import ImportPage from "./pages/ImportPage";
import TimelinePage from "./pages/TimelinePage";
import CategoryPage from "./pages/CategoryPage";
import SettingsPage from "./pages/SettingsPage";
import { api } from "./lib/ipc";
import type { Card, Category, Settings } from "./lib/types";
import { errorText } from "./lib/types";

function Shell() {
  const toast = useToast();
  const [page, setPage] = useState<PageKey>("import");
  const [cards, setCards] = useState<Card[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [editing, setEditing] = useState<Card | null>(null);
  const [pendingPaths, setPendingPaths] = useState<string[] | null>(null);

  const reload = useCallback(async () => {
    try {
      const [c, cats, s] = await Promise.all([
        api.listCards(),
        api.listCategories(),
        api.getSettings(),
      ]);
      setCards(c);
      setCategories(cats);
      setSettings(s);
    } catch (e) {
      toast.err("读取数据失败", errorText(e));
    }
  }, [toast]);

  useEffect(() => {
    void reload();
  }, [reload]);

  // 右键「发送到 循迹」启动时带了文件路径，这里取出来交给导入页处理。
  useEffect(() => {
    api
      .takePendingImports()
      .then((paths) => {
        if (paths.length > 0) {
          setPendingPaths(paths);
          setPage("import");
        }
      })
      .catch(() => undefined);
  }, []);

  const onChanged = (updated: Card) => {
    setCards((prev) => prev.map((c) => (c.id === updated.id ? updated : c)));
  };

  const relink = async (card: Card) => {
    const picked = await open({
      multiple: false,
      directory: false,
      title: `为「${card.displayName}」重新选择文件`,
    });
    if (!picked || Array.isArray(picked)) return;
    try {
      const updated = await api.relinkCard(card.id, picked);
      onChanged(updated);
      toast.ok("已重新定位", "说明和分类都保留着");
    } catch (e) {
      toast.err("重新定位失败", errorText(e));
    }
  };

  return (
    <div className="app">
      <Sidebar page={page} onNavigate={setPage} cardCount={cards.length} />
      <div className="content">
        <TitleBar />
        <main className="main">
          {page === "import" && (
            <ImportPage
              settings={settings}
              categories={categories}
              onSettingsChange={setSettings}
              onCategoriesChange={setCategories}
              onImported={reload}
              pendingPaths={pendingPaths}
              onPendingDone={() => setPendingPaths(null)}
            />
          )}

          {page === "timeline" && (
            <TimelinePage
              cards={cards}
              categories={categories}
              onOpenSummary={setEditing}
              onChanged={onChanged}
              onRelink={relink}
            />
          )}

          {page === "category" && (
            <CategoryPage
              cards={cards}
              categories={categories}
              onChanged={onChanged}
              onOpenCard={setEditing}
            />
          )}

          {page === "settings" && (
            <SettingsPage settings={settings} onSettingsChange={setSettings} />
          )}
        </main>
      </div>

      {editing && (
        <SummaryEditor
          card={editing}
          categories={categories}
          onClose={() => setEditing(null)}
          onSaved={(c) => {
            onChanged(c);
            void reload();
          }}
        />
      )}
    </div>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <Shell />
    </ToastProvider>
  );
}
