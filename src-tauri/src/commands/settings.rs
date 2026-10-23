//! 设置相关命令。
//!
//! API Key 存系统凭据库（`keyring`），不进 settings 表、不进配置文件、
//! 不出现在任何命令的返回值里（FR-020）。

use tauri::State;
use tauri_plugin_autostart::ManagerExt;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{AiConfigInput, Settings};
use crate::state::{AppState, KEYRING_ACCOUNT, KEYRING_SERVICE};

/// 应用运行时信息——数据存放位置、数据库路径、程序路径。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub data_dir: String,
    pub db_path: String,
    pub thumb_dir: String,
    pub exe_path: String,
    pub version: String,
}

/// 读取应用运行时信息。
#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> AppResult<AppInfo> {
    Ok(AppInfo {
        data_dir: state.data_dir.to_string_lossy().to_string(),
        db_path: state.db_path().to_string_lossy().to_string(),
        thumb_dir: state.thumb_dir().to_string_lossy().to_string(),
        exe_path: std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// 取得凭据库条目。keyring 3.x 的 `Entry::new` 返回 `Result`。
fn entry() -> AppResult<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|e| AppError::Io(format!("访问凭据库失败：{e}")))
}

/// 读取设置（不含 API Key）。
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    state.with_db(db::load_settings)
}

/// 保存设置（不含 API Key）。
#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> AppResult<Settings> {
    state.with_db(|conn| {
        db::save_settings(conn, &settings)?;
        db::load_settings(conn)
    })
}

/// 保存 AI 配置。
///
/// `api_key` 为空串表示不改动已存 key。返回的设置里不含 key。
#[tauri::command]
pub fn set_ai_config(
    state: State<'_, AppState>,
    config: AiConfigInput,
) -> AppResult<Settings> {
    let provider = config.provider.trim().to_string();
    let provider_name = config.provider_name.trim().to_string();
    let base_url = config.base_url.trim().trim_end_matches('/').to_string();
    let model = config.model.trim().to_string();

    if provider.is_empty() {
        return Err(AppError::Invalid("请填写 Provider ID".into()));
    }
    let mut provider_chars = provider.chars();
    let valid_provider = provider_chars
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase())
        && provider_chars
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
        && !provider.ends_with('-');
    if !valid_provider {
        return Err(AppError::Invalid(
            "Provider ID 只能使用小写字母、数字、连字符和下划线，并且必须以字母开头".into(),
        ));
    }
    if provider_name.is_empty() {
        return Err(AppError::Invalid("请填写供应商显示名称".into()));
    }
    if base_url.is_empty() || !(base_url.starts_with("https://") || base_url.starts_with("http://"))
    {
        return Err(AppError::Invalid(
            "API 地址必须以 http:// 或 https:// 开头".into(),
        ));
    }
    if model.is_empty() {
        return Err(AppError::Invalid("请填写模型名称".into()));
    }
    if !(0.0..=2.0).contains(&config.temperature) {
        return Err(AppError::Invalid("温度必须在 0 到 2 之间".into()));
    }

    let current = state.with_db(db::load_settings)?;
    let key = config.api_key.trim().to_string();
    if provider != current.ai_provider && key.is_empty() && has_api_key()? {
        return Err(AppError::Invalid(
            "更换提供商后请重新填写 API Key，避免误用旧密钥".into(),
        ));
    }

    // 先写凭据库：写失败就不要改 settings，避免出现"配置显示已保存但实际没 key"。
    if !key.is_empty() {
        entry()?
            .set_password(&key)
            .map_err(|e| AppError::Io(format!("保存 API Key 失败：{e}")))?;
    }

    state.with_db(|conn| {
        let mut s = db::load_settings(conn)?;
        s.ai_provider = provider;
        s.ai_provider_name = provider_name;
        s.ai_protocol = config.protocol;
        s.ai_base_url = base_url;
        s.ai_model = model;
        s.ai_temperature = config.temperature;
        s.ai_filename_only = config.filename_only;
        db::save_settings(conn, &s)?;
        db::load_settings(conn)
    })
}

/// 读取已存 API Key 是否存在——只返回布尔值，绝不返回 key 本身。
#[tauri::command]
pub fn has_api_key() -> AppResult<bool> {
    match entry() {
        Ok(e) => Ok(e.get_password().is_ok()),
        Err(_) => Ok(false),
    }
}

/// 查询是否已设置开机自启。
#[tauri::command]
pub fn get_autostart(app: tauri::AppHandle) -> AppResult<bool> {
    app.autolaunch()
        .is_enabled()
        .map_err(|e| AppError::Io(format!("读取开机自启状态失败：{e}")))
}

/// 设置开机自启。
#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> AppResult<bool> {
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|e| AppError::Io(format!("设置开机自启失败：{e}")))?;
    app.autolaunch()
        .is_enabled()
        .map_err(|e| AppError::Io(format!("读取开机自启状态失败：{e}")))
}

/// 清除已存 API Key。
#[tauri::command]
pub fn clear_api_key() -> AppResult<()> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Io(format!("清除 API Key 失败：{e}"))),
    }
}

/// 读取 API Key（仅供 Rust 侧发起 AI 请求使用，不暴露给前端）。
pub fn read_api_key() -> AppResult<String> {
    entry()?.get_password().map_err(|_| AppError::AiUnavailable)
}
