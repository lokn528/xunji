//! Tauri 命令层。
//!
//! 每个子模块是一组薄编排函数：解析参数 → 调用 `db` / `fsops` / `ai` 等纯逻辑
//! → 返回可序列化结果。业务规则不写在这一层。

pub mod ai;
pub mod cards;
pub mod categories;
pub mod files;
pub mod import;
pub mod settings;
