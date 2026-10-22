# Data Model: 循迹 v1

**Feature**: 循迹 v1 本地文件卡片索引
**Date**: 2026-10-22
**Storage**: SQLite 单文件，位于 `%APPDATA%/循迹/xunji.db`

## 概览

循迹的持久化数据分两处：

1. **SQLite**——卡片、分类、设置（非敏感部分）。备份即复制该文件。
2. **系统凭据库**——仅 API Key。绝不入库、绝不入配置文件。

缩略图是文件而非数据，存于缩略图缓存目录，由数据库中的相对路径索引；缓存被清
可重建，不影响卡片数据。

## ER 概览

```text
┌──────────────┐         ┌──────────────┐
│    cards     │         │  categories  │
├──────────────┤         ├──────────────┤
│ id      PK   │──┐      │ id      PK   │
│ display_name │  │      │ name    UNI  │
│ summary      │  └─────▶│ created_at   │
│ file_path UNI│         │ sort_order   │
│ thumb_path   │         └──────────────┘
│ category_id  │
│ origin       │
│ ai_generated │
│ needs_summary│
│ missing      │
│ pinned       │
│ created_at   │
│ updated_at   │
└──────────────┘

┌──────────────┐
│   settings   │  (key-value)
├──────────────┤
│ key       PK │
│ value        │
└──────────────┘
```

`cards.category_id` 可空（未分类），外键指向 `categories.id`。删除分类时该字段
置空（`ON DELETE SET NULL`），卡片本身保留。

## 表：cards

一张卡片 = 一件被收录事物的身份记录。核心是 `file_path`。

| 列 | 类型 | 约束 | 说明 |
|---|---|---|---|
| id | TEXT | PK | UUID v4 |
| display_name | TEXT | NOT NULL | 卡片显示名，默认取文件名，可改 |
| summary | TEXT | NOT NULL DEFAULT '' | 一句话说明；可暂空 |
| file_path | TEXT | NOT NULL UNIQUE | 指向磁盘实体的绝对路径 |
| thumb_path | TEXT | NULL | 缩略图绝对路径，可空 |
| category_id | TEXT | NULL, FK→categories.id | 单选分类，可空 |
| origin | TEXT | NOT NULL | `imported_moved` / `imported_kept` / `sent_to` |
| ai_generated | INTEGER | NOT NULL DEFAULT 0 | 说明是否由 AI 生成（0/1） |
| needs_summary | INTEGER | NOT NULL DEFAULT 1 | 说明为空待写（0/1） |
| missing | INTEGER | NOT NULL DEFAULT 0 | 源文件已丢失（0/1） |
| pinned | INTEGER | NOT NULL DEFAULT 0 | 置顶"近期要用"（0/1） |
| created_at | TEXT | NOT NULL | ISO 8601 导入时间，时间线依据 |
| updated_at | TEXT | NOT NULL | ISO 8601 |

**索引**：
- `UNIQUE(file_path)` —— 重复检测的结构性保证（FR-018）
- `idx_cards_created_at (created_at DESC)` —— 时间线分组
- `idx_cards_category (category_id)` —— 分类视图
- `idx_cards_pinned (pinned)` —— 置顶筛选

**取值域**：
- `origin` ∈ {`imported_moved`, `imported_kept`, `sent_to`}
- 布尔列以 INTEGER 0/1 存储

### 字段说明

- `summary` 与 `needs_summary` 是一对：说明为空时 `needs_summary=1`，写入说明后置 0。
  这样"待写"是可查询的状态，而不���靠 `summary == ''` 判断（避免用户手写空串的歧义）。
- `missing` 是**缓存状态**而非事实：每次列表查询时对 `file_path` 做存在性检查并回写。
  源文件恢复后自动清除标记。卡片永不因文件丢失而删除（constitution 原则 V）。
- `file_path` 在「移动」模式下指向新位置，在「仅登记」模式下指向原处。两者都指向
  文件当前所在的绝对路径。

## 表：categories

| 列 | 类型 | 约束 | 说明 |
|---|---|---|---|
| id | TEXT | PK | UUID v4 |
| name | TEXT | NOT NULL UNIQUE | 分类名，如"数学""毕设" |
| created_at | TEXT | NOT NULL | ISO 8601 |
| sort_order | INTEGER | NOT NULL DEFAULT 0 | 手动排序用 |

分类名唯一：归类到不存在的分类时先按名创建（FR-014）。

## 表：settings

键值表，存放非敏感设置。

| 列 | 类型 | 约束 |
|---|---|---|
| key | TEXT | PK |
| value | TEXT | NOT NULL |

**预置键**：

| key | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `storage.default_dir` | TEXT | 空 | 默认存储位置（绝对路径） |
| `storage.placement_mode` | TEXT | `move` | `move`（移动/复制到默认位置）/ `keep`（仅登记） |
| `ai.provider` | TEXT | `openai_compatible` | 供应商标识 |
| `ai.base_url` | TEXT | 空 | OpenAI 兼容 Base URL |
| `ai.model` | TEXT | 空 | 模型名 |
| `ai.temperature` | REAL | `0.3` | 采样温度 |
| `ai.filename_only` | INTEGER | `1` | "仅用文件名分析"开关，默认开（constitution 原则 IV） |
| `thumb.cache_dir` | TEXT | `<appdata>/thumbs` | 缩略图缓存目录 |

**不存于此表**：`ai.api_key` —— 存于系统凭据库（FR-020）。

## 凭据库条目

`keyring` crate，服务名 `xunji`，账户名 `ai_api_key`。应用只通过
`get_credential` / `set_credential` / `delete_credential` 三个命令访问，
读出的值不落盘、不进日志、不返回给前端之外的任何位置。

## 派生值（不落库）

- **打开所在位置** —— 以 `file_path` 调用系统文件管理器并选中该文件。随时可算。
- **文件存在性** —— 列表查询时实时检查 `file_path`，结果写入 `missing` 列作为缓存。
- **分组键** —— 时间线按 `created_at` 的日期部分分组，分类按 `category_id` 分组。
  都在查询层或前端实时算出，不冗余存储。

## 约束与不变量

1. `cards.file_path` 全局唯一 —— 一个文件在系统内只对应一张卡片。
2. `cards.summary` 可空但 `needs_summary` 必填 —— 两者状态一致。
3. 删除分类不删除卡片（`category_id` 置空）。
4. 删除卡片不删除磁盘上的文件本体 —— 循迹不做文件删除（v1 范围外）。
5. 导入是原子的：落盘 + 建卡在同一事务语义下，要么都成功，要么都不发生
   （constitution 原则 III 与开发流程约束）。

## 迁移

用 `rusqlite` 的 `user_version` PRAGMA 做版本管理：

| version | 内容 |
|---|---|
| 0 | 初始库（含 cards / categories / settings 三表与全部索引） |

每次启动检查 `user_version`，低于期望版本则执行迁移后再开连接。迁移语句在
Rust 侧以 `include_str!` 嵌入，保证编译期与应用版本一致。
