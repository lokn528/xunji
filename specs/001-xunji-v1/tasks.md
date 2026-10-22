# Tasks: 循迹 v1 本地文件卡片索引

**Input**: Design documents from `/specs/001-xunji-v1/`

**Prerequisites**: plan.md, spec.md, data-model.md, contracts/ipc-contract.md

**Tests**: Rust 侧用 `cargo test`（卡片 CRUD、状态持久化、AI 响应解析、路径处理与
重定位、重复检测）；前端用 `vitest`。UI 视觉与 Shell 菜单注册手工验一次。

**Organization**: 任务按用户故事分组，每个故事可独立实现与独立验证。

## Format

`[ID] [P?] [Story] 描述` —— `[P]` 表示可并行（不同文件、无依赖）；`[Story]` 标注
所属用户故事。描述中给出确切文件路径。

## Phase 1: 基础设施

**Purpose**: 项目骨架与共享基建，所有用户故事都依赖它

- [ ] T001 用 `npm create tauri-app` 脚手架生成 Tauri 2 + React + TypeScript 项目，落在仓库根目录（`src-tauri/`、`src/`、`package.json`），保留现有 `specs/` 与 `.specify/`
- [ ] T002 配置 `src-tauri/tauri.conf.json`：应用标识 `com.xunji.app`、窗口标题「循迹」、最小尺寸、窗口图标
- [ ] T003 配置 `src-tauri/Cargo.toml` 依赖：`tauri` 2、`tauri-plugin-dialog`、`tauri-plugin-opener`、`tauri-plugin-store`、`rusqlite`(bundled)、`reqwest`(rustls, json)、`keyring`、`serde`、`serde_json`、`uuid`、`chrono`、`image`、`thiserror`
- [ ] T004 配置 `src-tauri/capabilities/default.json`，授予前端调用自定义命令与 dialog 插件的权限
- [ ] T005 建立 `src-tauri/src/error.rs`：`AppError` 枚举 + `serde` 序列化，前端可读错误消息（区分磁盘满、无权限、源丢失、AI 不可用）
- [ ] T006 建立 `src-tauri/src/models.rs`：`Card`、`Category`、`Settings`、`Origin`、`ImportRequest`、`ImportResult` 结构体，与 data-model.md 字段一致
- [ ] T007 建立 `src-tauri/src/db.rs`：SQLite 连接、`user_version` 迁移、cards/categories/settings 三表与全部索引
- [ ] T008 写 `src-tauri/src/db.rs` 的建表与迁移单元测试（用内存库），验证表结构与索引存在
- [ ] T009 建立前端 `src/lib/types.ts`（与 Rust 模型对齐的 TS 类型）与 `src/lib/ipc.ts`（`invoke` 封装）
- [ ] T010 建立 `src/styles/global.css` 与 `src/App.tsx` 骨架：侧边栏（导入/时间线/分类/设置）+ 页面容器 + 基础路由状态
- [ ] T011 配置 `vitest` 与 `cargo test` 可运行；写一条冒烟测试打通前后端调用（Rust 命令返回固定值 → 前端渲染）

**Checkpoint**: 应用可启动，四个页面有壳，Rust 与前端互相能通气

---

## Phase 2: 共享核心（阻塞所有用户故事）

**Purpose**: 卡片与分类的数据操作，被多个故事共用

- [ ] T012 实现 `src-tauri/src/db.rs` 的卡片查询：`list_cards`（按 created_at 倒序，含置顶优先）、`get_card`、`update_card_summary`、`set_card_category`、`set_card_pinned`、`relink_card`
- [ ] T013 实现 `src-tauri/src/commands/cards.rs` 对应的 Tauri 命令层（薄编排，调用 db）
- [ ] T014 实现 `src-tauri/src/db.rs` 的分类查询：`list_categories`（含每类卡片数）、`create_category`、`delete_category`（置空卡片 category_id）
- [ ] T015 实现 `src-tauri/src/commands/categories.rs`
- [ ] T016 写卡片 CRUD 单元测试：增删改查、置顶切换、分类归类与置空、字段持久化后重新打开库仍可读
- [ ] T017 写重复检测单元测试：同一 file_path 再次插入被唯一索引拒绝，且不产生第二行
- [ ] T018 写路径处理单元测试：Windows 路径规范化、大小写与分隔符差异、同一文件的等价路径识别为重复

**Checkpoint**: 卡片与分类的增删改查有测试护着，重启后数据不丢

---

## Phase 3: 用户故事 1 —— 丢进来的文件自动变成一张卡片 (P1)

**Goal**: 拖拽或右键导入文件，落到设定位置并建卡；文件夹整体建卡；重复与失败都有明确行为

**Why this first**: 这是产品存在的理由，没有它后面全部无从谈起

**Independent Test**: 拖入一个文件 → 存储位置出现该文件、卡片列表出现对应卡片、路径指向磁盘真实文件；重启后卡片仍在

- [ ] T019 实现 `src-tauri/src/fsops.rs`：文件大小判定、目标路径生成（重名不覆盖）、移动与复制、错误映射（磁盘满/无权限/跨卷）
- [ ] T020 实现 `src-tauri/src/fsops.rs` 的落点模式分支：`move`（移动）、`keep`（仅登记不动）、>100MB 时仅登记不复制
- [ ] T021 实现 `src-tauri/src/commands/import.rs` 的导入编排：落盘 → 建卡 → 事务提交；落盘失败不建卡，建卡失败则回滚落盘（移回原处或删副本）
- [ ] T022 实现导入的去重：按规范化 file_path 查重，命中则返回 `AlreadyExists` 与既有卡片 id，前端提示「已收录」并跳转该卡
- [ ] T023 实现文件夹导入：整体生成一张卡片，附带子项列表（相对路径 + 大小 + 类型），子项不单独建卡
- [ ] T024 实现 `src-tauri/src/shellmenu.rs` 与 `scripts/register-shell-menu.ps1`：注册 `shell\\open\\command` 的「发送到 循迹」，支持安装与卸载
- [ ] T025 实现 Tauri 命令 `import_from_paths`（接收拖拽/右键传来的路径列表），并让应用在启动时接收命令行参数作为待导入项
- [ ] T026 实现前端 `src/components/DropZone.tsx` 与 `src/pages/ImportPage.tsx`：拖拽投放区、落点临时覆盖选择、导入结果逐条反馈
- [ ] T027 实现前端导入成功/失败提示组件（`src/components/Toast.tsx`），错误文案取自 Rust 侧可读错误
- [ ] T028 写导入编排单元测试：正常导入建卡、磁盘满报错且无卡片、>100MB 仅登记、重复导入返回既有 id

**Checkpoint**: 拖入文件 → 卡片出现 → 重启仍在。可独立演示

---

## Phase 4: 用户故事 2 —— 给卡片补上「这是干嘛的」 (P1)

**Goal**: AI 生成说明与建议分类，先进输入框待确认；失败一律降级不阻塞

**Why this priority**: 补上说明是循迹区别于普通收藏夹的核心差异

**Independent Test**: 空说明卡片点「让 AI 写说明」→ 输入框出现 AI 结果；未配 key 时提示不可用且卡片完好

- [ ] T029 实现 `src-tauri/src/ai.rs` 的 OpenAI 兼容请求构造：Base URL + 模型 + 温度 + 消息体，按 `ai.filename_only` 开关裁剪输入内容
- [ ] T030 实现 AI 响应解析：提取一句话说明（≤50 字）与建议分类；解析失败返回可读错误，绝不写入半截内容
- [ ] T031 实现 AI 调用的错误分级：未配置 key → `AiUnavailable`；网络/超时 → `AiRequestFailed`；返回异常 → `AiInvalidResponse`；三者都不得影响卡片本身
- [ ] T032 实现 `src-tauri/src/commands/ai.rs` 的 `analyze_card` 命令（按卡片 id 组装输入）与 `test_ai_connection` 命令
- [ ] T033 实现凭据库读写：`src-tauri/src/commands/settings.rs` 中的 API Key 存/取/删（`keyring`，服务名 `xunji`），确保 key 不出现在返回值与日志中
- [ ] T034 实现前端 `src/components/SummaryEditor.tsx`：AI 结果填入可编辑输入框 + 建议分类下拉，确认后才调命令落库
- [ ] T035 在卡片上接入「让 AI 写说明」按钮；AI 结果未确认前不写入卡片（constitution 原则 II）
- [ ] T036 实现「导入后立即 AI 分析」勾选项（默认不勾选），并确保 AI 失败时导入仍算成功
- [ ] T037 写 AI 响应解析单元测试：正常响应、说明超长截断、返回非 JSON、返回空内容、建议分类不在已有分类中
- [ ] T038 写请求体裁剪单元测试：开关开启时请求体不含文件内容；关闭时只含允许的文本片段（constitution 原则 IV 的结构性保证）
- [ ] T039 写 AI 失败降级单元测试：三种错误各自返回对应错误类型，卡片记录不受影响

**Checkpoint**: 手动触发 AI → 建议进输入框 → 确认后落卡。未配 key 时一切照常。可独立演示

---

## Phase 5: 用户故事 3 —— 顺着卡片找回文件 (P1)

**Goal**: 时间线与分类视图、打开文件、定位所在目录、置顶、源文件丢失可重新定位

**Why this priority**: 找回是「收集 → 理解 → 找回」闭环的最后一环，缺它产品不成立

**Independent Test**: 导入若干文件 → 时间线按日期分组 → 点「位置」打开资源管理器并选中文件；移走源文件后卡片标记丢失且可重定位

- [ ] T040 实现 `src-tauri/src/commands/files.rs`：`open_file`（系统默认程序打开）、`reveal_in_folder`（打开所在目录并选中文件）
- [ ] T041 实现源文件存在性检查：列表查询时逐个检查 `file_path` 并回写 `missing` 标记，文件恢复后自动清除
- [ ] T042 实现 `relink_card`：用户重新选择文件后绑定回原卡片，保留原 id、说明与分类
- [ ] T043 实现 `src-tauri/src/thumb.rs`：图片类文件生成缩略图存入缓存目录，非图片类返回 None（v1 不做 PDF 首页缩略）
- [ ] T044 实现前端 `src/pages/TimelinePage.tsx`：按日期分组、组内倒序、置顶卡片浮于顶部
- [ ] T045 实现前端 `src/pages/CategoryPage.tsx`：按分类分组网格、显示各分类卡片数、点击分类过滤
- [ ] T046 实现前端 `src/components/CardTile.tsx`：缩略图、显示名、说明、分类徽标、日期、置顶按钮、丢失标记与「重新定位」入口
- [ ] T047 实现卡片点击行为：主体打开文件，「位置」按钮打开所在目录并选中（卡片上需可区分两者）
- [ ] T048 实现「源文件丢失」卡片的视觉区分与「重新定位」交互（原生文件选择器）
- [ ] T049 写路径处理与重定位单元测试：文件移动后卡片标记丢失、重新绑定后 id 与说明保留、绑定到已被别的卡片占用的路径时报错

**Checkpoint**: 时间线/分类可浏览，点卡片能打开与定位，丢失文件能找回。可独立演示

---

## Phase 6: 用户故事 4 —— 管理分类 (P2)

**Goal**: 分类页可建分类、可归类、归到新分类自动创建

**Why this priority**: 分类提升找回效率，但无分类时靠时间线仍能工作

**Independent Test**: 建两个分类各归入若干卡片 → 分类页见两个分组及数量 → 点击可过滤

- [ ] T050 实现前端分类管理：创建分类、删除分类（卡片置空不删）、卡片上切换分类
- [ ] T051 实现 `src/components/CategoryChip.tsx`：分类徽标、当前分类高亮、归类下拉（可输入新分类名）
- [ ] T052 实现空分类状态的一键创建入口
- [ ] T053 写分类单元测试：重名分类被拒（唯一约束）、删除分类后卡片保留且 category_id 为空

**Checkpoint**: 分类可建可归可删，卡片不因分类删除而丢失

---

## Phase 7: 用户故事 5 —— 配置默认行为与 AI 供应商 (P2)

**Goal**: 默认存储位置、落点模式、AI 供应商配置与连通性测试、开机自启

**Why this priority**: 设置让体验贴合个人习惯，但无设置也能用默认值先跑起来

**Independent Test**: 改默认存储位置后导入文件落到新位置；填 API 配置后点测试连通性得到明确反馈

- [ ] T054 实现 `src-tauri/src/commands/settings.rs`：settings 键值读写，默认值注入（`ai.filename_only` 默认 1、`placement_mode` 默认 `move`）
- [ ] T055 实现默认存储位置未配置时的降级：提示用户先设置，或退回到用户主目录下的 `循迹` 目录（不得静默失败）
- [ ] T056 实现前端 `src/pages/SettingsPage.tsx`：目录选择器、落点模式单选、AI 供应商配置表单（含 API Key 密码框）、连通性测试按钮
- [ ] T057 实现 API Key 输入框的显示/隐藏切换，且不将明文 key 写进任何日志或错误消息
- [ ] T058 实现开机自启设置项（`tauri-plugin-autostart`，若不可用则以注册表 Run 键实现）
- [ ] T059 实现缩略图缓存目录设置项与「重建缩略图缓存」动作
- [ ] T060 写设置持久化单元测试：写入后重开库仍可读、API Key 不出现在 settings 表中

**Checkpoint**: 设置可改并持久生效，key 不落盘

---

## Phase 8: 打磨与验证

**Purpose**: 跨卡片的收尾与质量守门

- [ ] T061 写 `specs/001-xunji-v1/quickstart.md` 手工验证手册，覆盖设计文档第 11 节的走查清单
- [ ] T062 端到端手工走查：拖拽导入 → 落点 → 建卡 → AI 说明 → 时间线/分类检索 → 打开文件/所在位置 → 置顶 → 改默认位置 → 重启数据仍在
- [ ] T063 走查边界情况：磁盘满/无权限、重复导入、>100MB 文件、源文件被删、分类被删、AI 断网
- [ ] T064 性能检查：100 张卡片的时间线首屏渲染 < 1s，导入 100 文件 < 30s，内存占用 < 150MB
- [ ] T065 检查数据文件与日志，确认无明文 API Key、无文件内容外发痕迹
- [ ] T066 右键「发送到 循迹」注册与卸载各手工验一次
- [ ] T067 跑 `cargo test` 与 `vitest` 全绿，`cargo clippy` 无告警
- [ ] T068 写 README：开发环境要求（Rust / Node / MSVC）、构建运行方式、数据存放位置、备份方式、右键菜单安装脚本用法

---

## Dependencies 与执行顺序

### 阶段依赖

- **Phase 1 基础设施**：无依赖，立即开始
- **Phase 2 共享核心**：依赖 Phase 1
- **Phase 3/4/5（三个 P1）**：均依赖 Phase 1 + Phase 2，彼此间尽量解耦，可并行
- **Phase 6/7（P2）**：依赖 Phase 2 + 对应的 P1
- **Phase 8 打磨**：依赖全部

### 用户故事依赖

- US1（建卡）→ 无前置，但 US2、US3 都以卡片存在为前提
- US2（AI 说明）→ 依赖 US1 建卡；不依赖 US3
- US3（找回）→ 依赖 US1 建卡；US4 分类可独立于 US3
- US4（分类）→ 依赖 Phase 2 的分类数据层
- US5（设置）→ 依赖 Phase 1/2，AI 供应商部分与 US2 共享命令

### 并行机会

- T003–T004（依赖配置）与 T005–T008（Rust 核心）可并行
- T009–T010（前端骨架）与 T005–T008（Rust 核心）可并行
- T012/T013、T014/T015 可并行
- Phase 3（T019–T028）、Phase 4（T029–T039）、Phase 5（T040–T049）三段可由多人/多轮并行推进
- Phase 6（T050–T053）与 Phase 7（T054–T060）可并行

### MVP 切分

1. 完成 Phase 1 + 2 → 基础设施就绪
2. 完成 Phase 3 → **MVP 可交付**：拖入文件变成卡片，重启仍在
3. 叠加 Phase 5 → 核心闭环成立：收集 → 找回
4. 叠加 Phase 4 → 完整价值：收集 → 理解 → 找回
5. 叠加 Phase 6/7 → 个性化可用

## Notes

- 每个 `[P]` 任务涉及不同文件，可并行推进
- 提交策略：每个任务完成即提交，提交信息用 `feat:` / `fix:` / `test:` 前缀
- T028、T037–T039、T049、T053、T060 是 constitution 要求的自动化测试，必须实写，不得跳过
