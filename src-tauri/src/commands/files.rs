//! 文件打开与定位相关命令。

use std::path::Path;

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// 用系统默认程序打开文件。
#[tauri::command]
pub fn open_file(app: AppHandle, path: String) -> AppResult<()> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(AppError::SourceMissing(path));
    }
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| AppError::Io(format!("打开文件失败：{e}")))
}

/// 打开文件所在文件夹并选中该文件。
#[tauri::command]
pub fn reveal_in_folder(app: AppHandle, path: String) -> AppResult<()> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(AppError::SourceMissing(path));
    }
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| AppError::Io(format!("定位文件失败：{e}")))
}

/// 列出文件夹的直接子项——文件夹卡片的"展开子项"用。
#[tauri::command]
pub fn list_folder_children(
    _state: State<'_, AppState>,
    path: String,
) -> AppResult<Vec<crate::models::FolderChild>> {
    let root = Path::new(&path);
    if !root.is_dir() {
        return Err(AppError::Invalid("不是一个文件夹".into()));
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(root)?;
    for entry in entries.flatten() {
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_dir = meta.is_dir();
        let rel = entry
            .path()
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push(crate::models::FolderChild {
            relative_path: rel,
            size: meta.len(),
            extension: crate::fsops::extension(&entry.path().to_string_lossy()),
            is_dir,
        });
    }
    out.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(out)
}
