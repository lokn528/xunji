-- 循迹 v1 初始 schema（user_version = 0）
--
-- 卡片以 file_path 为核心：它指向磁盘上的实体，是"找回"的凭据。
-- file_path 上的唯一约束是重复导入检测的结构性保证。

CREATE TABLE IF NOT EXISTS categories (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL UNIQUE,
    created_at  TEXT NOT NULL,
    sort_order  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS cards (
    id            TEXT PRIMARY KEY NOT NULL,
    display_name  TEXT NOT NULL,
    summary       TEXT NOT NULL DEFAULT '',
    file_path     TEXT NOT NULL UNIQUE,
    thumb_path    TEXT,
    category_id   TEXT REFERENCES categories(id) ON DELETE SET NULL,
    origin        TEXT NOT NULL,
    ai_generated  INTEGER NOT NULL DEFAULT 0,
    needs_summary INTEGER NOT NULL DEFAULT 1,
    missing       INTEGER NOT NULL DEFAULT 0,
    pinned        INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cards_created_at ON cards(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_cards_category    ON cards(category_id);
CREATE INDEX IF NOT EXISTS idx_cards_pinned      ON cards(pinned);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
