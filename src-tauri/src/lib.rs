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

use std::path::Path;

use rusqlite::{Connection, OpenFlags};
use tauri::{webview::PageLoadEvent, Manager};

use crate::error::AppResult;
use crate::state::AppState;

#[cfg(windows)]
fn apply_rounded_window_corners(window: &tauri::WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    };

    if let Ok(hwnd) = window.hwnd() {
        // Windows 11 原生圆角；旧系统不支持时会安全地忽略。
        let preference = DWMWCP_ROUND;
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &preference as *const _ as *const std::ffi::c_void,
                std::mem::size_of_val(&preference) as u32,
            );
        }
    }
}

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
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let window = webview.window();
                let _ = window.show();
                let _ = window.set_focus();
            }
        })
        .setup(|app| {
            let legacy_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("xunji"));

            #[cfg(debug_assertions)]
            let data_dir = legacy_dir.clone();

            #[cfg(not(debug_assertions))]
            let data_dir = {
                let exe = std::env::current_exe()?;
                exe.parent()
                    .map(Path::to_path_buf)
                    .ok_or("无法取得程序所在目录")?
            };

            std::fs::create_dir_all(&data_dir)?;
            migrate_legacy_data(&legacy_dir, &data_dir)?;

            let conn = db::open(&data_dir.join("xunji.db"))?;
            initialize_app_defaults(&conn, &data_dir)?;
            let state = AppState::new(conn, data_dir);
            // 右键「发送到 循迹」会把选中的文件路径作为命令行参数传进来。
            state.set_pending_imports(pending_import_paths());
            app.manage(state);

            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("main") {
                apply_rounded_window_corners(&window);
            }

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
            commands::settings::get_autostart,
            commands::settings::set_autostart,
            commands::ai::analyze_card,
            commands::ai::analyze_path,
            commands::ai::test_ai_connection,
            shellmenu::install_shell_menu,
            shellmenu::uninstall_shell_menu,
            shellmenu::shell_menu_installed,
            commands::import::take_pending_imports,
        ])
        .run(tauri::generate_context!())
        .expect("启动循迹失败");
}

/// 补齐首次启动应有的默认值：存储目录与常用分类。
fn initialize_app_defaults(conn: &Connection, data_dir: &Path) -> AppResult<()> {
    db::seed_default_categories(conn)?;

    let mut settings = db::load_settings(conn)?;
    if settings.default_dir.trim().is_empty() {
        let library = data_dir.join("library");
        std::fs::create_dir_all(&library)?;
        settings.default_dir = library.to_string_lossy().to_string();
        db::save_settings(conn, &settings)?;
    }
    Ok(())
}

/// 发布版以程序目录为数据目录，旧版 AppData 数据只向新目录合并。
fn migrate_legacy_data(
    legacy_dir: &Path,
    data_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if legacy_dir == data_dir {
        return Ok(());
    }

    let legacy_db = legacy_dir.join("xunji.db");
    let data_db = data_dir.join("xunji.db");
    if !data_db.exists() && legacy_db.exists() {
        copy_sqlite_database(&legacy_db, &data_db)?;
    }

    let legacy_thumbs = legacy_dir.join("thumbs");
    let data_thumbs = data_dir.join("thumbs");
    copy_missing_tree(&legacy_thumbs, &data_thumbs)?;

    if data_db.exists() {
        rewrite_thumb_paths(&data_db, &data_thumbs)?;
    }
    Ok(())
}

/// 用 SQLite 在线备份复制数据库；先写临时文件，完成后再原子改名。
fn copy_sqlite_database(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let temp = target.with_extension("db.migrating");
    if temp.exists() {
        std::fs::remove_file(&temp)?;
    }
    {
        let mut destination = Connection::open(&temp)?;
        let backup = rusqlite::backup::Backup::new(&source, &mut destination)?;
        backup.run_to_completion(64, std::time::Duration::from_millis(5), None)?;
    }
    std::fs::rename(temp, target)?;
    Ok(())
}

/// 合并旧缩略图目录，同名文件以目标目录现有版本为准。
fn copy_missing_tree(source: &Path, target: &Path) -> std::io::Result<()> {
    if !source.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_missing_tree(&entry.path(), &destination)?;
        } else if file_type.is_file() && !destination.exists() {
            std::fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

/// 卡片里存的是绝对缩略图路径；整个程序目录移动后也改到当前目录。
fn rewrite_thumb_paths(
    db_path: &Path,
    data_thumbs: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = Connection::open(db_path)?;
    let changes = {
        let mut statement =
            conn.prepare("SELECT id, thumb_path FROM cards WHERE thumb_path IS NOT NULL")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut changes = Vec::new();
        for row in rows {
            let (id, old_path) = row?;
            if let Some(file_name) = Path::new(&old_path).file_name() {
                let new_path = data_thumbs.join(file_name).to_string_lossy().to_string();
                if new_path != old_path {
                    changes.push((id, new_path));
                }
            }
        }
        changes
    };

    if changes.is_empty() {
        return Ok(());
    }
    let transaction = conn.transaction()?;
    for (id, thumb_path) in changes {
        transaction.execute(
            "UPDATE cards SET thumb_path = ?2 WHERE id = ?1",
            rusqlite::params![id, thumb_path],
        )?;
    }
    transaction.commit()?;
    Ok(())
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
    fn 首次启动会补齐默认分类和存储目录() {
        let root = tempfile::tempdir().unwrap();
        let conn = db::open_in_memory().unwrap();

        initialize_app_defaults(&conn, root.path()).unwrap();

        let categories = db::list_categories(&conn).unwrap();
        let names = categories
            .iter()
            .map(|category| category.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["学习", "工作", "生活", "资料"]);

        let settings = db::load_settings(&conn).unwrap();
        let expected = root.path().join("library");
        assert_eq!(Path::new(&settings.default_dir), expected);
        assert!(expected.is_dir(), "默认存储目录应被创建");
    }

    #[test]
    fn 旧版数据库与缩略图会迁移到程序目录() {
        let root = tempfile::tempdir().unwrap();
        let legacy = root.path().join("legacy");
        let data = root.path().join("portable");
        std::fs::create_dir_all(legacy.join("thumbs")).unwrap();
        std::fs::create_dir_all(&data).unwrap();

        let old_thumb = legacy.join("thumbs").join("cover.png");
        std::fs::write(&old_thumb, b"thumb").unwrap();

        let old_db = db::open(&legacy.join("xunji.db")).unwrap();
        old_db
            .execute(
                "INSERT INTO cards
                   (id, display_name, summary, file_path, thumb_path, category_id,
                    origin, ai_generated, needs_summary, missing, pinned,
                    created_at, updated_at, is_dir)
                 VALUES ('old-1', '旧卡片', '', 'C:\\docs\\cover.png', ?1, NULL,
                         'imported_kept', 0, 1, 0, 0,
                         '2026-10-22T00:00:00.000Z', '2026-10-22T00:00:00.000Z', 0)",
                rusqlite::params![old_thumb.to_string_lossy()],
            )
            .unwrap();
        drop(old_db);

        migrate_legacy_data(&legacy, &data).unwrap();

        let new_thumb = data.join("thumbs").join("cover.png");
        assert!(new_thumb.exists(), "缩略图应复制到新目录");
        let new_db = Connection::open(data.join("xunji.db")).unwrap();
        let (id, thumb): (String, String) = new_db
            .query_row("SELECT id, thumb_path FROM cards", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(id, "old-1");
        assert_eq!(Path::new(&thumb), new_thumb, "卡片应指向新缩略图路径");
    }

    #[test]
    fn 启动参数解析过滤掉不存在的路径() {
        let paths = pending_import_paths();
        assert!(paths.iter().all(|p| std::path::Path::new(p).exists()));
    }
}
