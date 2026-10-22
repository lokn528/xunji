# 快速上手：循迹 v1

**日期**: 2026-10-22
**Feature**: 循迹 v1 本地文件卡片索引

## 一句话

把丢过的文件变成一张卡：一句说明 + 分类 + 日期。点一下就回到它在电脑里的位置。

## 开发环境

| 依赖 | 版本 | 备注 |
|---|---|---|
| Rust | stable（≥ 1.85） | edition 2024 需要 |
| Node.js | ≥ 20 | 实测 24.17 |
| MSVC Build Tools | 任意近期版本 | 需含「使用 C++ 的桌面开发」工作负载 |
| Windows SDK | 10.0.26100 | Build Tools 附带，装不全会缺 kernel32.lib |
| WebView2 | 运行时 v155 | Windows 11 自带 |

缺 MSVC 或 Windows SDK 时，`cargo build` 会在链接阶段报
`can't find crate for std` 或 `linker link.exe not found`。

## 跑起来

```bash
npm install
npm run tauri dev
```

## 打包

```bash
npm run tauri build
```

产物在 `src-tauri/target/release/bundle/` 下（NSIS 安装包 + MSI）。

## 跑测试

```bash
cd src-tauri && cargo test     # Rust 侧：58 项单元测试
npx vitest run                 # 前端：13 项单元测试
npx tsc --noEmit               # 前端类型检查
```

## 打包可能遇到的坑

### 1. `can't find crate for std`

Rust 工具链装到一半被中断。修复：

```bash
rustup toolchain uninstall stable
rustup toolchain install stable --profile minimal
rustup component add clippy rustfmt
```

### 2. `link.exe not found` / 缺 kernel32.lib

MSVC 编译工具装了但没装 Windows SDK。修复：装 Build Tools 时加上
「使用 C++ 的桌面开发」工作负载，缺 SDK 会报缺 `kernel32.lib`。

### 3. NSIS 打包报「系统无法将文件移到不同的磁盘驱动器」

tauri-bundler 把 NSIS 解压到项目 `target/release` 再重命名到
`%LOCALAPPDATA%\tauri\NSIS`，跨盘时 `rename` 会失败。解法是**预先把 NSIS 工具
放好**，让打包器跳过下载与解压（它只检查 13 个必需文件是否齐全）：

```powershell
$cache = "$env:LOCALAPPDATA\tauri"

# NSIS 本体
curl.exe -L -o nsis.zip "https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.13/nsis-3.13.zip"
Expand-Archive nsis.zip nsisx -Force
New-Item -ItemType Directory -Force -Path "$cache\NSIS" | Out-Null
robocopy nsisx\nsis-3.13 "$cache\NSIS" /MIR

# Tauri 自己的 NSIS 插件（单独下载，NSIS_REQUIRED_FILES 里列了它）
curl.exe -L -o "$cache\NSIS\Plugins\x86-unicode\additional\nsis_tauri_utils.dll" `
  "https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll"
```

国内网络下载 GitHub 资产慢或超时，可以设镜像：

```powershell
$env:TAURI_BUNDLER_TOOLS_GITHUB_MIRROR = "https://你的镜像前缀"
```

## 数据在哪

| 内容 | 位置 |
|---|---|
| 数据库 | `%APPDATA%\app.xunji.desktop\xunji.db` |
| 缩略图缓存 | `%APPDATA%\app.xunji.desktop\thumbs\` |
| API Key | Windows 凭据库（服务名 `xunji`，账户名 `ai_api_key`） |

备份 = 复制 `xunji.db`。缩略图缓存可以随时清，会自动重建。

## 注册右键「发送到 循迹」

在应用内的「设置 → 系统」点按钮，或手动执行：

```powershell
.\scripts\register-shell-menu.ps1 -ExePath "src-tauri\target\release\xunji.exe"
```

移除：

```powershell
.\scripts\register-shell-menu.ps1 -Uninstall
```

只写 `HKCU`，不需要管理员权限。

## 手工走查清单

按顺序走一遍，覆盖 P1 的三条主线：

1. **设置默认存储位置** — 设置页选一个目录，保存。
2. **拖拽导入** — 把 2~3 个文件拖进导入页。检查：文件落到了设定位置、
   卡片出现在列表、说明显示「待写」。
3. **仅登记模式** — 切到「仅登记，原地不动」再拖一个文件。检查：原文件没动。
4. **右键导入** — 资源管理器里右键一个文件 → 发送到 → 循迹。
5. **重复导入** — 把同一个文件再拖一次。检查：提示「已收录」，卡片没有变成两张。
6. **AI 说明** — 卡片点「说明」→「让 AI 写说明」。检查：结果先进输入框，
   点保存才落库。改一个字后保存，检查 AI 标记消失。
7. **AI 不可用** — 设置里清掉 API Key，再点「让 AI 写说明」。检查：提示
   「AI 不可用，可手动填写」，卡片和已有数据不受影响。
8. **时间线** — 切到时间线。检查：按日期分组、组内倒序、置顶卡片浮在顶部。
9. **找回文件** — 点卡片的「位置」。检查：资源管理器打开并选中了那个文件。
10. **分类** — 建一个分类，给卡片归类。检查：分类页出现分组与数量。
11. **源文件丢失** — 把某个文件从磁盘删掉，刷新。检查：卡片标为「源文件丢失」，
    点「重新定位」能绑回新文件，且说明和分类都还在。
12. **重启** — 关掉应用再打开。检查：卡片、分类、设置都还在。

## 边界情况怎么验

| 场景 | 怎么做 | 期望 |
|---|---|---|
| 磁盘满 / 无权限 | 目标目录填一个不存在的路径层级 | 明确报错，不产生半成品卡片 |
| 大文件（>100MB） | 造一个大文件，用「移动」模式导入 | 只登记路径，不复制内容 |
| 源文件无读取权限 | 把文件权限设为不可读 | 仍建卡片，无缩略图 |
| 缩略图缓存被清 | 删掉 thumbs 目录 | 卡片数据不受影响，缩略图可重建 |
| AI 返回格式异常 | 指向一个返回乱码的接口 | 提示解析失败，不写入半截内容 |

## 隐私自查

```powershell
# 数据库里不应出现 API Key
Select-String -Path "$env:APPDATA\app.xunji.desktop\xunji.db" -Pattern "sk-" -SimpleMatch
```

应无输出。

## 不测的部分

- UI 视觉样式
- Shell 菜单注册（手工验一次即可）
