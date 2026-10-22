# 循迹

> 所有丢过的东西，有迹可循。

给丢过的每个文件发一张「户口卡」——一句话说明 + 截图 + 分类 + 日期，随时能顺着
卡片找回它在电脑里的确切位置。

循迹**不是文件管理器**。它不接管你的文件存储，只做参谋：文件本体始终留在本机，
数据库只存路径与元数据。

## 特性

- **拖拽即建卡** —— 文件或文件夹拖进来就变成一张卡片；也可以在资源管理器里
  右键「发送到 > 循迹」。
- **一句话说明** —— 手动写，或让 AI 根据文件名给出建议（先进输入框，你确认后才落库）。
- **时间线与分类** —— 按日期或分类找回，点「位置」在资源管理器里直接定位到那个文件。
- **源文件丢了也不怕** —— 卡片标记「源文件丢失」，可重新定位，保留原有说明与分类。
- **隐私边界清楚** —— 文件永不上传；AI Key 存 Windows 凭据库；可开「仅用文件名分析」，
  连文件内容都不外发。

## 技术栈

| 层 | 选型 |
|---|---|
| 桌面壳 | Tauri 2（Rust） |
| 前端 | React 19 + TypeScript + Vite |
| 存储 | SQLite（`rusqlite`，bundled，单文件） |
| AI | OpenAI 兼容的 `/chat/completions`（rustls） |
| 凭据 | Windows 凭据库（`keyring`） |

架构上有一条硬线：**文件操作、数据库访问与 AI 请求全在 Rust 侧**。前端只通过
命令交互，不碰 SQLite、不持有 API Key、不构造 AI 请求体。这让「文件本体不出本机」
和「最小内容发送」成为结构性保证，而不是靠前端自觉。

## 开发

需要 Rust（≥ 1.85，edition 2024）、Node ≥ 20、MSVC Build Tools（含「使用 C++ 的
桌面开发」工作负载）与 Windows SDK。

```bash
npm install
npm run tauri dev        # 开发模式
npm run tauri build      # 打包（产物在 src-tauri/target/release/bundle/）
```

测试：

```bash
cd src-tauri && cargo test
npx tsc --noEmit
```

## 数据存放

| 内容 | 位置 |
|---|---|
| 数据库 | `%APPDATA%\循迹\xunji.db` |
| 缩略图缓存 | `%APPDATA%\循迹\thumbs\` |
| API Key | Windows 凭据库（服务名 `xunji`） |

备份就是复制 `xunji.db`。缩略图缓存可随时清，会自动重建。

## 右键菜单

应用内「设置 → 系统」一键注册，或手动：

```powershell
.\scripts\register-shell-menu.ps1 -ExePath "src-tauri\target\release\xunji.exe"
.\scripts\register-shell-menu.ps1 -Uninstall
```

只写 `HKCU`，不需要管理员权限，免额外常驻进程。

## 项目结构

```text
src-tauri/src/
├── db.rs        # SQLite 访问与迁移
├── models.rs    # 领域模型
├── fsops.rs     # 路径规范化、移动/复制
├── ai.rs        # OpenAI 兼容请求构造与响应解析
├── thumb.rs     # 缩略图生成
├── textprobe.rs # 可提取文本片段
├── shellmenu.rs # 右键菜单注册
└── commands/    # Tauri 命令层（薄编排）

src/
├── lib/         # IPC 封装与 TS 类型
├── components/  # 卡片、侧边栏、说明编辑器、提示
└── pages/       # 导入 / 时间线 / 分类 / 设置

specs/001-xunji-v1/   # 规格、计划、数据模型、任务清单、走查手册
```

## 边界

v1 明确不做：文件内容编辑、云同步、账号体系、全文检索、AI 自动归档决策、
浏览器插件。链接收集由「淘金客」承担。

## 许可

私有项目。
