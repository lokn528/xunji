//! 分类相关命令。

use tauri::State;

use crate::db;
use crate::error::AppResult;
use crate::models::Category;
use crate::state::AppState;

/// 查询全部分类，附各分类卡片数。
#[tauri::command]
pub fn list_categories(state: State<'_, AppState>) -> AppResult<Vec<Category>> {
    state.with_db(db::list_categories)
}

/// 创建分类。重名时报错。
#[tauri::command]
pub fn create_category(state: State<'_, AppState>, name: String) -> AppResult<Category> {
    state.with_db(|conn| db::create_category(conn, &name))
}

/// 删除分类。其下卡片保留，分类置空。
#[tauri::command]
pub fn delete_category(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.with_db(|conn| db::delete_category(conn, &id))
}
