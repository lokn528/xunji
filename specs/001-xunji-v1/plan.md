# Implementation Plan: 循迹 v1 本地文件卡片索引

**Branch**: `001-xunji-v1` | **Date**: 2026-10-22 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-xunji-v1/spec.md`

## Summary

交付「循迹」v1 首个可用切片：一个 Windows 桌面应用，把用户丢过的文件变成可检索的
"户口卡"。Tauri 2 + Rust 承载桌面壳与全部文件操作，React + TypeScript 承载界面，
本地 SQLite 存卡片与设置。用户在导入页拖入文件或经资源管理器右键"发送到"导入，
应用按设置落盘并建卡；卡片可通过时间线/分类找回；可手动或经 AI 补写一句话说明。

技术路径直接对应设计文档第 8 节的推荐方案（除 AI 调用位置由前端直连改为 Rust 侧
发起，理由见下）。所有文件操作——移动、复制、打开、在资源管理器中定位——都在
Rust 侧完成，AI 密钥同样由 Rust 侧持有。

## Technical Context

**Language/Version**: Rust 1.8x (stable, 2024+ edition)；TypeScript 5.x

**Primary Dependencies**:
- `tauri` 2.x（桌面壳）
- `tauri-plugin-dialog`（目录选择器、文件选择器）
- `tauri-plugin-opener` 或 `tauri-plugin-shell`（打开文件、定位所在目录）
- `tauri-plugin-store` 或自建 settings 表（设置持久化）
- `rusqlite`（bundled feature，避免额外 SQLite 安装步骤）
- `reqwest`（OpenAI 兼容接口调用，rustls 避免 OpenSSL 依赖）
- `keyring`（Windows 凭据库存 API Key）
- `serde` / `serde_json`（序列化）
- React 18 + TypeScript + Vite（前端）
- `tauri-plugin-sql` 或经 Rust 命令代理的 `rusqlite`（数据访问）

**Storage**: 本地 SQLite 单文件，位于 Tauri 应用数据目录（`%APPDATA%/循迹/xunji.db`）

**Testing**: `cargo test`（Rust 单元测试：路径处理、重复检测、AI 响应解析、缩略图
生成）；`vitest`（前端单元测试）；手工走查（UI 视觉、Shell 菜单注册）

**Target Platform**: Windows 11 x64（WebView2 v155 已具备）

**Project Type**: desktop-app

**Performance Goals**: 100 张卡片时时间线首屏渲染 < 1s；IPC 单次调用 < 50ms；
导入 100 个文件总耗时 < 30s

**Constraints**: 内存占用 < 150MB（区别于 Electron 的数量级差距）；文件本体绝不出
本机；数据文件不含明文 API Key

**Scale/Scope**: 单用户单机型；v1 约 4 个页面、5 个用户故事、1 个数据表族

## Constitution Check

对照 `.specify/memory/constitution.md` 逐条核验：

| 原则 | 落地方式 | 结论 |
|---|---|---|
| I. 文件本体永不出本机 | SQLite 只存路径与元数据；无任何上传代码路径；AI 请求体在 Rust 侧构造并可断言不含文件字节 | 通过 |
| II. AI 只建议，绝不自动决策 | AI 返回值只写入前端待确认输入框；落库须经用户显式保存动作 | 通过 |
| III. AI 失败绝不阻塞主流程 | 建卡与 AI 调用解耦：先落卡，再可选触发 AI；AI 错误降级为提示 | 通过 |
| IV. 最小内容发送 | 开关默认"仅用文件名"；Rust 侧构造请求体时按开关裁剪字段 | 通过 |
| V. 卡片以路径为唯一真实来源 | `file_path` 唯一索引；丢失只标记不删除；重新定位走显式绑定动作 | 通过 |

技术约束核验：Tauri 2 + Rust ✓；SQLite 单文件 ✓；文件操作在 Rust 侧 ✓；
API Key 存系统凭据库（`keyring` crate）✓；OpenAI 兼容 `/chat/completions` ✓；
Windows 优先单机、无账号体系 ✓。

开发流程核验：卡片 CRUD/状态持久化、AI 响应解析、路径处理与重定位、重复检测
均有 Rust 单元测试 ✓；导入失败明确报错且无半成品卡片由事务保证 ✓。

**GATE PASSED** —— 无需 Complexity Tracking 豁免项。

## Project Structure

### Documentation (this feature)

```text
specs/001-xunji-v1/
├── plan.md              # 本文件
├── spec.md              # 需求规格
├── research.md          # Phase 0：技术选型与关键决策
├── data-model.md        # Phase 1：SQLite 模式
├── quickstart.md        # Phase 1：手工验证手册
├── contracts/
│   └── ipc-contract.md  # Phase 1：Rust ⇄ 前端命令契约
├── checklists/
│   └── requirements.md  # 规格质量清单
└── tasks.md             # Phase 2：任务清单
```

### Source Code (repository root)

```text
src-tauri/
├── Cargo.toml
├── tauri.conf.json
├── build.rs
├── capabilities/
│   └── default.json
├── icons/
└── src/
    ├── main.rs           # 应用入口、插件注册
    ├── lib.rs            # 库入口，run() 供 main 与测试共用
    ├── error.rs          # 统一错误类型（可序列化为前端可读消息）
    ├── db.rs             # SQLite 连接、迁移、查询
    ├── models.rs         # Card / Category / Settings 结构体
    ├── commands/
    │   ├── mod.rs
    │   ├── cards.rs      # 卡片 CRUD、置顶、重定位
    │   ├── import.rs     # 导入编排、落点模式、重复检测
    │   ├── files.rs      # 打开文件、定位所在目录、缩略图
    │   ├── categories.rs # 分类 CRUD
    │   ├── settings.rs   # 设置读写、凭据库
    │   └── ai.rs         # OpenAI 兼容调用、响应解析
    ├── fsops.rs          # 移动/复制/大小判定/错误映射
    ├── thumb.rs          # 图片缩略图生成
    ├── textprobe.rs      # 可提取文本片段（PDF/文本类）
    └── shellmenu.rs      # 右键"发送到 循迹"注册表写入

src/                     # React 前端
├── main.tsx
├── App.tsx               # 侧边栏 + 路由
├── styles/
│   └── global.css
├── lib/
│   ├── ipc.ts            # tauri invoke 封装与类型
│   └── types.ts          # 与 Rust 共享的 TS 类型定义
├── components/
│   ├── Sidebar.tsx
│   ├── CardTile.tsx
│   ├── CardList.tsx
│   ├── SummaryEditor.tsx # AI 建议待确认输入框
│   ├── DropZone.tsx
│   ├── CategoryChip.tsx
│   └── Toast.tsx
└── pages/
    ├── ImportPage.tsx
    ├── TimelinePage.tsx
    ├── CategoryPage.tsx
    └── SettingsPage.tsx

scripts/
└── register-shell-menu.ps1  # 右键菜单注册/卸载脚本

tests/                   # 前端单元测试
└── *.test.ts
```

**Structure Decision**: 单一 Tauri 项目，`src-tauri/`（Rust 侧）与 `src/`（React 侧）
并列于仓库根目录，与 Tauri 官方脚手架一致。业务逻辑分层：`fsops`/`thumb`/
`textprobe` 是无 UI 依赖的纯函数模块（便于单元测试），`commands/` 是薄编排层，
`db.rs` 独占 SQL。前端不直接持有密钥，也不直接构造 AI 请求体——两者都在 Rust 侧，
以满足 constitution 的 I 与 IV。

## 关键技术决策

### AI 调用放在 Rust 侧而非前端

设计文档第 8 节给出"Rust 或前端直连"两个选项。选 Rust 侧，理由有三：
API Key 需要存入系统凭据库（前端 JS 沙箱拿不到凭据库）；前端直连会让密钥进入
WebView 内存并可能经 devtools 暴露；Rust 侧构造请求体可在此处强制裁剪字段，
使"最小内容发送"成为结构性保证而非前端自觉。

### 数据访问经 Rust 命令代理

不用 `tauri-plugin-sql` 让前端直连 SQLite。卡片查询、重复检测、事务性导入都
需要跨多表的一致性判断，放在 Rust 侧可复用同一套规则并保证导入的原子性
（失败时不留半成品卡片）。前端只调用命令。

### 导入事务边界

一次导入 = 一次事务：落盘（移动/复制）→ 建卡 → 提交。落盘失败则回滚且不建卡；
落盘成功但建卡失败则把文件移回原处（移动模式）或删除副本（复制模式），不留下
孤儿文件。file_path 上建唯一索引，使并发导入同一文件时最终只留一张卡。

### 缩略图 v1 只做图片类

图片类文件用 `image` crate 直接生成缩略图；PDF 首页缩略图需要额外引入 PDF 渲染
依赖，v1 先不做，PDF 卡片允许用户手动粘贴截图。已在 Assumptions 中记录。

## Complexity Tracking

无。Constitution Check 全部通过，无豁免项。
