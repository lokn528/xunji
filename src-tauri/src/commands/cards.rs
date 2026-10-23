//! 卡片相关命令。

use std::path::Path;

use tauri::State;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::Card;
use crate::state::AppState;

/// 查询全部卡片，并刷新源文件存在性标记。
#[tauri::command]
pub fn list_cards(state: State<'_, AppState>) -> AppResult<Vec<Card>> {
    state.with_db(|conn| {
        let mut cards = db::list_cards(conn)?;
        for card in &mut cards {
            let exists = Path::new(&card.file_path).exists();
            if exists != !card.missing {
                db::set_missing(conn, &card.id, !exists)?;
                card.missing = !exists;
            }
        }
        Ok(cards)
    })
}

/// 按 id 查询一张卡片。
#[tauri::command]
pub fn get_card(state: State<'_, AppState>, id: String) -> AppResult<Option<Card>> {
    state.with_db(|conn| db::get_card(conn, &id))
}

/// 写入说明。`ai_generated` 标记该说明是否由 AI 生成。
#[tauri::command]
pub fn set_summary(
    state: State<'_, AppState>,
    id: String,
    summary: String,
    ai_generated: bool,
) -> AppResult<Card> {
    state.with_db(|conn| {
        db::update_summary(conn, &id, &summary, ai_generated)?;
        db::get_card(conn, &id)?
            .ok_or_else(|| AppError::Invalid("卡片不存在".into()))
    })
}

/// 修改显示名。
#[tauri::command]
pub fn set_display_name(
    state: State<'_, AppState>,
    id: String,
    display_name: String,
) -> AppResult<Card> {
    state.with_db(|conn| {
        db::update_display_name(conn, &id, &display_name)?;
        db::get_card(conn, &id)?
            .ok_or_else(|| AppError::Invalid("卡片不存在".into()))
    })
}

/// 设置卡片的多个分类。空数组表示取消分类；分类不存在时自动创建。
#[tauri::command]
pub fn set_categories(
    state: State<'_, AppState>,
    id: String,
    categories: Vec<String>,
) -> AppResult<Card> {
    state.with_db(|conn| {
        db::set_categories(conn, &id, &categories)?;
        db::get_card(conn, &id)?
            .ok_or_else(|| AppError::Invalid("卡片不存在".into()))
    })
}

/// 设置置顶状态。
#[tauri::command]
pub fn set_pinned(state: State<'_, AppState>, id: String, pinned: bool) -> AppResult<Card> {
    state.with_db(|conn| {
        db::set_pinned(conn, &id, pinned)?;
        db::get_card(conn, &id)?
            .ok_or_else(|| AppError::Invalid("卡片不存在".into()))
    })
}

/// 重新定位：把卡片绑定到新的文件路径，保留 id、说明与分类。
#[tauri::command]
pub fn relink_card(
    state: State<'_, AppState>,
    id: String,
    new_path: String,
) -> AppResult<Card> {
    let abs = crate::fsops::to_absolute(&new_path);
    if !Path::new(&abs).exists() {
        return Err(AppError::SourceMissing(abs));
    }
    state.with_db(|conn| {
        db::relink_card(conn, &id, &abs)?;
        db::get_card(conn, &id)?
            .ok_or_else(|| AppError::Invalid("卡片不存在".into()))
    })
}

/// 删除卡片。文件本体不受影响。
#[tauri::command]
pub fn delete_card(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.with_db(|conn| db::delete_card(conn, &id))
}
