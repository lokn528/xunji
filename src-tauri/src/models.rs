//! 领域模型。
//!
//! 字段与 `specs/001-xunji-v1/data-model.md` 一一对应。

use serde::{Deserialize, Serialize};

/// 导入方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// 移动/复制到默认位置。
    ImportedMoved,
    /// 仅登记，原地不动。
    ImportedKept,
    /// 经资源管理器右键「发送到 循迹」导入。
    SentTo,
}
impl Origin {
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::ImportedMoved => "imported_moved",
            Origin::ImportedKept => "imported_kept",
            Origin::SentTo => "sent_to",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "imported_kept" => Origin::ImportedKept,
            "sent_to" => Origin::SentTo,
            _ => Origin::ImportedMoved,
        }
    }
}

/// 一张卡片。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    pub display_name: String,
    /// 一句话说明。可为空。
    pub summary: String,
    /// 指向磁盘实体的绝对路径——卡片的核心。
    pub file_path: String,
    pub thumb_path: Option<String>,
    pub category_id: Option<String>,
    pub origin: Origin,
    /// 该说明是否由 AI 生成。
    pub ai_generated: bool,
    /// 说明为空、待写。
    pub needs_summary: bool,
    /// 源文件已丢失。
    pub missing: bool,
    /// 该卡片指向的是文件夹（前端据此提供「展开子项」入口）。
    pub is_dir: bool,
    /// 置顶「近期要用」。
    pub pinned: bool,
    /// ISO 8601，固定宽度以保证字典序即时间序。
    pub created_at: String,
    pub updated_at: String,
}

/// 一个分类。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub sort_order: i64,
    /// 该分类下的卡片数量。
    #[serde(default)]
    pub card_count: i64,
}

/// 落点模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementMode {
    /// 移动/复制到默认位置。
    Move,
    /// 仅登记，原地不动。
    Keep,
}
impl PlacementMode {
    pub fn as_str(self) -> &'static str {
        match self {
            PlacementMode::Move => "move",
            PlacementMode::Keep => "keep",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "keep" => PlacementMode::Keep,
            _ => PlacementMode::Move,
        }
    }
}

/// 导入请求。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    /// 待导入的源路径（文件或文件夹）。
    pub paths: Vec<String>,
    /// 临时覆盖的落点目录；为空则用设置中的默认位置。
    #[serde(default)]
    pub target_dir: Option<String>,
    /// 临时覆盖的落点模式；为空则用设置中的默认模式。
    #[serde(default)]
    pub placement: Option<PlacementMode>,
    /// 导入后是否立即触发 AI 分析。默认不触发。
    #[serde(default)]
    pub analyze_after: bool,
    /// 本批卡片统一归入的分类名；为空表示暂不分类。
    #[serde(default)]
    pub category: Option<String>,
    /// 本批卡片统一写入的备注说明；为空表示待写。
    #[serde(default)]
    pub note: Option<String>,
    /// 来源标记：右键「发送到」为 sent_to。
    #[serde(default)]
    pub origin: Option<Origin>,
}

/// 单项导入结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportItemResult {
    /// 源路径。
    pub source_path: String,
    /// 成功时的卡片 id。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_id: Option<String>,
    /// 重复导入时指向的既有卡片 id。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_card_id: Option<String>,
    /// 失败原因（可读中文）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 失败分类，供前端分支处理。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// AI 建议（仅当导入启用了 AI 分析且 AI 可用）。
    /// 依然是建议：前端须填入输入框，用户确认后才落库。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_suggestion: Option<AiSuggestion>,
}

/// 一次导入的汇总结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub items: Vec<ImportItemResult>,
    pub succeeded: usize,
    pub failed: usize,
    pub duplicates: usize,
}

/// 文件夹卡片下的子项。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderChild {
    /// 相对文件夹根的相对路径。
    pub relative_path: String,
    pub size: u64,
    /// 扩展名，不含点；无扩展名为空串。
    pub extension: String,
    pub is_dir: bool,
}

/// AI 分析的建议结果——先进输入框，用户确认后才落库。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSuggestion {
    /// 一句话说明，≤50 字。
    pub summary: String,
    /// 建议分类；用户现有分类之外的会被视为新分类建议。
    pub suggested_category: Option<String>,
    /// AI 原始返回，便于排障。
    pub raw: String,
}

/// 应用设置（非敏感部分）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// 默认存储位置（绝对路径）。
    pub default_dir: String,
    /// 落点模式。
    pub placement_mode: PlacementMode,
    /// AI 供应商标识。
    pub ai_provider: String,
    /// OpenAI 兼容 Base URL。
    pub ai_base_url: String,
    /// 模型名。
    pub ai_model: String,
    /// 采样温度。
    pub ai_temperature: f64,
    /// 「仅用文件名分析」开关，默认开（constitution 原则 IV）。
    pub ai_filename_only: bool,
    /// 缩略图缓存目录。
    pub thumb_cache_dir: String,
    /// 开机自启。
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            default_dir: String::new(),
            placement_mode: PlacementMode::Move,
            ai_provider: "openai_compatible".into(),
            ai_base_url: String::new(),
            ai_model: String::new(),
            ai_temperature: 0.3,
            ai_filename_only: true,
            thumb_cache_dir: String::new(),
            autostart: false,
        }
    }
}

/// 设置页的 AI 配置提交体。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfigInput {
    pub provider: String,
    pub base_url: String,
    /// 空串表示不改动已存 key。
    pub api_key: String,
    pub model: String,
    pub temperature: f64,
    pub filename_only: bool,
}

/// 生成固定宽度的 ISO 8601 时间戳。
///
/// 固定宽度保证字符串字典序与时间序一致，时间线分组与排序可直接依赖字典序。
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
