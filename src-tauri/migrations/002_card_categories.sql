-- 文件与分类改为多对多：同一张卡片可以拥有多个分类标签。
-- 旧的 cards.category_id 保留为历史列，数据在这里一次性迁入关联表。

CREATE TABLE IF NOT EXISTS card_categories (
    card_id     TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, category_id)
);

INSERT OR IGNORE INTO card_categories (card_id, category_id)
SELECT id, category_id
  FROM cards
 WHERE category_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_card_categories_category
    ON card_categories(category_id);
