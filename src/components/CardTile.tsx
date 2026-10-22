import { useState } from "react";
import { api, formatTime, thumbUrl } from "../lib/ipc";
import type { Card } from "../lib/types";
import { errorText } from "../lib/types";
import { useToast } from "./Toast";

const EXT_ICON: Record<string, string> = {
  pdf: "📕",
  doc: "📘",
  docx: "📘",
  xls: "📗",
  xlsx: "📗",
  ppt: "📙",
  pptx: "📙",
  txt: "📄",
  md: "📄",
  png: "🖼️",
  jpg: "🖼️",
  jpeg: "🖼️",
  gif: "🖼️",
  webp: "🖼️",
  mp4: "🎬",
  zip: "🗜️",
  rar: "🗜️",
};

function iconFor(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return EXT_ICON[ext] ?? "📄";
}

export default function CardTile({
  card,
  categoryName,
  onOpenSummary,
  onChanged,
  onRelink,
}: {
  card: Card;
  categoryName?: string;
  onOpenSummary: (card: Card) => void;
  onChanged: (card: Card) => void;
  onRelink: (card: Card) => void;
}) {
  const toast = useToast();
  const [busy, setBusy] = useState(false);
  const thumb = thumbUrl(card.thumbPath);

  const open = async () => {
    try {
      await api.openFile(card.filePath);
    } catch (e) {
      toast.err("打不开这个文件", errorText(e));
    }
  };

  const reveal = async () => {
    try {
      await api.revealInFolder(card.filePath);
    } catch (e) {
      toast.err("找不到文件位置", errorText(e));
    }
  };

  const togglePin = async () => {
    setBusy(true);
    try {
      const updated = await api.setPinned(card.id, !card.pinned);
      onChanged(updated);
    } catch (e) {
      toast.err("置顶失败", errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <article className={`card${card.missing ? " missing" : ""}`}>
      <div className="card-thumb">
        {thumb ? (
          <img src={thumb} alt="" />
        ) : (
          <span className="placeholder" aria-hidden="true">
            {iconFor(card.filePath)}
          </span>
        )}
      </div>

      <div className="card-body">
        <div className="card-title" title={card.displayName}>
          {card.displayName}
        </div>

        {card.summary ? (
          <div className="card-summary">{card.summary}</div>
        ) : (
          <div className="card-summary pending">待写说明</div>
        )}

        <div className="card-path" title={card.filePath}>
          {card.filePath}
        </div>

        <div className="card-meta">
          {categoryName ? (
            <span className="badge">{categoryName}</span>
          ) : (
            <span className="badge badge-plain">未分类</span>
          )}
          {card.aiGenerated && <span className="badge badge-plain">AI</span>}
          {card.missing && <span className="badge badge-danger">源文件丢失</span>}
          {card.pinned && <span className="badge badge-warn">置顶</span>}
          <span className="spacer" />
          <span className="muted" style={{ fontSize: 11 }}>
            {formatTime(card.createdAt)}
          </span>
        </div>

        <div className="card-actions">
          {card.missing ? (
            <button className="btn btn-sm" onClick={() => onRelink(card)}>
              重新定位
            </button>
          ) : (
            <>
              <button className="btn btn-sm" onClick={open}>
                打开
              </button>
              <button className="btn btn-sm" onClick={reveal}>
                位置
              </button>
            </>
          )}
          <button
            className="btn btn-sm"
            onClick={() => onOpenSummary(card)}
            title="填写或让 AI 生成说明"
          >
            说明
          </button>
          <span className="spacer" />
          <button
            className="btn-icon"
            onClick={togglePin}
            disabled={busy}
            title={card.pinned ? "取消置顶" : "置顶（近期要用）"}
          >
            {card.pinned ? "★" : "☆"}
          </button>
        </div>
      </div>
    </article>
  );
}
