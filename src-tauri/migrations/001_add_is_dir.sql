-- 文件夹卡片需要知道自己是文件夹，前端才能提供「展开子项」入口。

ALTER TABLE cards ADD COLUMN is_dir INTEGER NOT NULL DEFAULT 0;
