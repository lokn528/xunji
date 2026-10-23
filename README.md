# 循迹

> 所有丢过的东西，有迹可循。

给丢过的每个文件发一张「户口卡」——一句话说明 + 截图 + 分类 + 日期，随时能顺着
卡片找回它在电脑里的确切位置。

循迹**不是文件管理器**。它不接管你的文件存储，只做参谋：文件本体始终留在本机，
数据库只存路径与元数据。

## 怎么打开

循迹是 **Windows 桌面应用**，不是网页。双击 `dist\index.html` 或在浏览器里访问
只会看到一个说明页 —— 卡片存在本机 SQLite、文件操作与 AI 密钥都走 Rust 侧，
浏览器环境里跑不起来。

| 想做什么 | 怎么做 |
|---|---|
| 直接用 | 运行 `src-tauri\target\release\xunji.exe`（免安装，绿色运行） |
| 装到系统里 | 跑 `src-tauri\target\release\bundle\` 下的 MSI 或 NSIS 安装程序，装到 `%LOCALAPPDATA%\循迹\` |
| 开发调试 | 在项目目录执行 `npm run tauri dev` |

安装后：开始菜单有「循迹」快捷方式，卸载走「设置 → 应用 → 卸载」或
`%LOCALAPPDATA%\循迹\uninstall.exe`。

右键菜单用这个脚本注册/注销（指向哪个 exe 就注册哪个）：

```powershell
.\scripts\register-shell-menu.ps1 -ExePath "$env:LOCALAPPDATA\循迹\xunji.exe"
.\scripts\register-shell-menu.ps1 -Uninstall
```

## 特性

- **拖拽后确认导入** —— 文件或文件夹拖进待导入列表，可先用 AI 填好分类与备注，
  再确认建卡；也可以在资源管理器里右键「发送到 > 循迹」。
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

界面走 Nocturne Glassmorphism：深墨夜景底色 + 钢蓝光斑，玻璃面板是无色半透明白
（`rgba(255,255,255,.05~.12)` + `backdrop-blur(40px)` + `backdrop-saturate(180%)`），
阴影有方向（顶边受光 / 底边背光 / 外层深度），叠加 2.5% 噪点颗粒。唯一的强调色
是香槟金 `#E4B863`，只用在主 CTA、置顶态和待写标记上。

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
| 数据库 | 发布版 `xunji.exe` 同目录下的 `xunji.db` |
| 默认导入目录 | 发布版首次启动自动使用 `xunji.exe` 同目录下的 `library\` |
| 缩略图缓存 | 发布版 `xunji.exe` 同目录下的 `thumbs\` |
| API Key | Windows 凭据库（服务名 `xunji`） |

整个程序目录可以一起移动：发布版的数据跟着 `xunji.exe` 走。首次从旧版升级时，
程序会把 `%APPDATA%\app.xunji.desktop` 中已有的数据库和缩略图合并到新目录，原数据
不会删除。备份就是复制 `xunji.db`。缩略图缓存可随时清，会自动重建。

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
