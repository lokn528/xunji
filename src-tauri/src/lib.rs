//! 循迹 —— 本地文件卡片索引工具。
//!
//! 架构：Rust 侧持有数据库、文件系统操作与 AI 凭据；前端只通过命令交互。
//! 这样"文件本体不出本机"与"最小内容发送"是结构性保证，而非前端自觉。

mod ai;
mod commands;
mod db;
mod error;
mod fsops;
mod models;
mod shellmenu;
mod state;
mod textprobe;
mod thumb;

use std::sync::Mutex;

use tauri::Manager;

use crate::state::AppState;

/// 应用入口。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("xunji"));
            std::fs::create_dir_all(&data_dir)?;

            let conn = db::open(&data_dir.join("xunji.db"))?;
            app.manage(AppState {
                db: Mutex::new(conn),
                data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::cards::list_cards,
            commands::cards::get_card,
            commands::cards::set_summary,
            commands::cards::set_display_name,
            commands::cards::set_category,
            commands::cards::set_pinned,
            commands::cards::relink_card,
            commands::cards::delete_card,
            commands::categories::list_categories,
            commands::categories::create_category,
            commands::categories::delete_category,
            commands::files::open_file,
            commands::files::reveal_in_folder,
            commands::files::list_folder_children,
            commands::import::import_paths,
            commands::import::folder_children,
            commands::settings::get_settings,
            commands::settings::get_app_info,
            commands::settings::save_settings,
            commands::settings::set_ai_config,
            commands::settings::has_api_key,
            commands::settings::clear_api_key,
            commands::ai::analyze_card,
            commands::ai::test_ai_connection,
            shellmenu::install_shell_menu,
            shellmenu::uninstall_shell_menu,
            shellmenu::shell_menu_installed,
        ])
        .run(tauri::generate_context!())
        .expect("启动循迹失败");
}

/// 解析启动参数中待导入的路径（右键「发送到 循迹」会传入文件路径）。
pub fn pending_import_paths() -> Vec<String> {
    std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .filter(|a| std::path::Path::new(a).exists())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 启动参数解析过滤掉不存在的路径() {
        let paths = pending_import_paths();
        assert!(paths.iter().all(|p| std::path::Path::new(p).exists()));
    }
}
