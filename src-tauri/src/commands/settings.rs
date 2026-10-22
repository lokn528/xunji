//! 设置相关命令。
//!
//! API Key 存系统凭据库（`keyring`），不进 settings 表、不进配置文件、
//! 不出现在任何命令的返回值里（FR-020）。

use tauri::State;

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
    state.with_db(|conn| db::load_settings(conn))
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
    // 先写凭据库：写失败就不要改 settings，避免出现"配置显示已保存但实际没 key"。
    let key = config.api_key.trim().to_string();
    if !key.is_empty() {
        entry()?
            .set_password(&key)
            .map_err(|e| AppError::Io(format!("保存 API Key 失败：{e}")))?;
    }

    state.with_db(|conn| {
        let mut s = db::load_settings(conn)?;
        s.ai_provider = config.provider;
        s.ai_base_url = config.base_url;
        s.ai_model = config.model;
        s.ai_temperature = config.temperature;
        s.ai_filename_only = config.filename_only;
        db::save_settings(conn, &s)?;
        Ok(db::load_settings(conn)?)
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
