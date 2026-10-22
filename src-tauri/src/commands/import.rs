//! 导入编排。
//!
//! 事务边界：一次导入 = 落盘 → 建卡 → 提交。
//! 落盘失败则不建卡；建卡失败则把文件移回原处，不留下孤儿文件
//! （见 `plan.md` 的「导入事务边界」）。

use std::path::Path;

use tauri::State;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::fsops::{self, LandAction, Placement};
use crate::models::{
    Card, FolderChild, ImportItemResult, ImportRequest, ImportResult, Origin, PlacementMode,
};
use crate::state::AppState;

/// 导入一组路径。
#[tauri::command]
pub async fn import_paths(
    state: State<'_, AppState>,
    request: ImportRequest,
) -> AppResult<ImportResult> {
    let settings = state.with_db(db::load_settings)?;

    let placement = match request.placement {
        Some(p) => p,
        None => settings.placement_mode,
    };
    let target_dir = request
        .target_dir
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| settings.default_dir.clone());
    let origin = request.origin.unwrap_or(match placement {
        PlacementMode::Keep => Origin::ImportedKept,
        PlacementMode::Move => Origin::ImportedMoved,
    });

    let mut result = ImportResult {
        items: Vec::new(),
        succeeded: 0,
        failed: 0,
        duplicates: 0,
    };

    for source in &request.paths {
        let item = import_one(
            &state,
            source,
            &target_dir,
            placement,
            origin,
        );
        match &item {
            Ok(r) if r.existing_card_id.is_some() => result.duplicates += 1,
            Ok(_) => result.succeeded += 1,
            Err(_) => result.failed += 1,
        }
        let mut converted = item_to_result(source, item);

        // 导入后立即 AI 分析：产出建议但不写库（constitution 原则 II）。
        // AI 失败不影响导入结果——卡片已建好，说明留空可手写。
        if request.analyze_after {
            if let Some(card_id) = converted.card_id.clone() {
                match super::ai::analyze_card_inner(&state, &card_id).await {
                    Ok(s) => converted.ai_suggestion = Some(s),
                    Err(_) => { /* 静默降级：卡片照常建好 */ }
                }
            }
        }
        result.items.push(converted);
    }

    Ok(result)
}

/// 导入单个源路径。
///
/// 返回 `Ok` 且 `existing_card_id` 非空表示"已收录"。
fn import_one(
    state: &AppState,
    source: &str,
    target_dir: &str,
    placement: PlacementMode,
    origin: Origin,
) -> AppResult<ImportItemResult> {
    let abs_source = fsops::to_absolute(source);
    let source_path = Path::new(&abs_source);

    if !source_path.exists() {
        return Err(AppError::SourceMissing(abs_source));
    }
    let is_dir = source_path.is_dir();

    // 落点目录：Move 模式必须有目标目录。Keep 模式不需要。
    let placement = if placement == PlacementMode::Move && target_dir.trim().is_empty() {
        return Err(AppError::Invalid(
            "还没有设置默认存储位置，请先到设置页选择".into(),
        ));
    } else {
        placement
    };

    // 重复检测：按规范化路径比对（大小写与分隔符差异视为同一文件）。
    let normalized = fsops::normalize_path(&abs_source);
    let existing = state.with_db(|conn| {
        for card in db::list_cards(conn)? {
            if fsops::normalize_path(&card.file_path) == normalized {
                return Ok(Some(card.id));
            }
        }
        Ok(None)
    })?;
    if let Some(id) = existing {
        return Ok(ImportItemResult {
            source_path: abs_source,
            card_id: None,
            existing_card_id: Some(id),
            error: None,
            error_code: None,
            ai_suggestion: None,
        });
    }

    // 规划并执行落盘。
    let plan = fsops::plan_land(
        &abs_source,
        target_dir,
        match placement {
            PlacementMode::Move => Placement::Move,
            PlacementMode::Keep => Placement::Keep,
        },
        is_dir,
    );

    let (final_path, moved_to) = match plan {
        LandAction::None => (abs_source.clone(), None),
        LandAction::Move(target) => {
            if is_dir {
                fsops::move_dir(&abs_source, &target)?;
            } else {
                fsops::move_file(&abs_source, &target)?;
            }
            (target.to_string_lossy().to_string(), Some(target))
        }
    };

    // 建卡。失败则回滚落盘，不留孤儿文件。
    let card = build_card(&abs_source, &final_path, origin, is_dir);
    let thumb = state
        .thumb_dir()
        .to_string_lossy()
        .to_string();
    let thumb_path = crate::thumb::generate(&final_path, &thumb)
        .map(|p| p.to_string_lossy().to_string());
    let mut card = card;
    card.thumb_path = thumb_path;

    let insert = state.with_db(|conn| {
        db::insert_card(conn, &card)?;
        Ok(())
    });

    if let Err(e) = insert {
        // 回滚：把文件移回原处
        if let Some(target) = moved_to {
            let _ = rollback(&abs_source, &target, is_dir);
        }
        return Err(e);
    }

    Ok(ImportItemResult {
        source_path: abs_source,
        card_id: Some(card.id),
        existing_card_id: None,
        error: None,
        error_code: None,
        ai_suggestion: None,
    })
}

/// 建卡失败时把已落盘的文件移回原处。
fn rollback(original: &str, target: &Path, is_dir: bool) -> AppResult<()> {
    if is_dir {
        fsops::move_dir(&target.to_string_lossy(), Path::new(original))
    } else {
        fsops::move_file(&target.to_string_lossy(), Path::new(original))
    }
}

/// 构造一张新卡片。
fn build_card(source: &str, final_path: &str, origin: Origin, is_dir: bool) -> Card {
    let now = crate::models::now_iso();
    let display_name = if is_dir {
        fsops::file_name(source)
    } else {
        fsops::file_name(final_path)
    };
    Card {
        id: uuid::Uuid::new_v4().to_string(),
        display_name,
        summary: String::new(),
        file_path: final_path.to_string(),
        thumb_path: None,
        category_id: None,
        origin,
        ai_generated: false,
        // 说明留空 → 标记为待写（FR-006）
        needs_summary: true,
        missing: false,
        // 文件夹卡片带 is_dir，前端据此提供「展开子项」入口（FR-025）
        is_dir,
        pinned: false,
        created_at: now.clone(),
        updated_at: now,
    }
}

/// 把内部结果转成对外结构，并把错误转成可读文案。
fn item_to_result(
    source: &str,
    outcome: AppResult<ImportItemResult>,
) -> ImportItemResult {
    match outcome {
        Ok(r) => r,
        Err(e) => {
            let code = e.code().to_string();
            ImportItemResult {
                source_path: source.to_string(),
                card_id: None,
                existing_card_id: None,
                error: Some(e.to_string()),
                error_code: Some(code),
                ai_suggestion: None,
            }
        }
    }
}

/// 查询一个文件夹的直接子项（供前端展开文件夹卡片）。
#[tauri::command]
pub fn folder_children(path: String) -> AppResult<Vec<FolderChild>> {
    let root = Path::new(&path);
    if !root.is_dir() {
        return Err(AppError::Invalid("不是一个文件夹".into()));
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root)?.flatten() {
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let rel = entry
            .path()
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push(FolderChild {
            relative_path: rel,
            size: meta.len(),
            extension: fsops::extension(&entry.path().to_string_lossy()),
            is_dir: meta.is_dir(),
        });
    }
    out.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(out)
}

// ----------------------------------------------------------------- 测试

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn make_state(dir: &Path) -> AppState {
        AppState {
            db: Mutex::new(db::open_in_memory().unwrap()),
            data_dir: dir.to_path_buf(),
        }
    }

    #[test]
    fn 导入文件会建卡并移动到目标目录() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        let src = src_dir.join("note.txt");
        std::fs::write(&src, "hello").unwrap();

        let state = make_state(dir.path());
        let item = import_one(
            &state,
            src.to_str().unwrap(),
            dst_dir.to_str().unwrap(),
            PlacementMode::Move,
            Origin::ImportedMoved,
        )
        .unwrap();

        assert!(item.card_id.is_some());
        assert!(!src.exists(), "源文件应被移走");
        assert!(dst_dir.join("note.txt").exists(), "应落到目标目录");

        let cards = state.with_db(db::list_cards).unwrap();
        assert_eq!(cards.len(), 1);
        assert!(cards[0].needs_summary, "说明应标记待写");
        assert_eq!(cards[0].file_path, dst_dir.join("note.txt").to_string_lossy());
    }

    #[test]
    fn 仅登记模式原文件不动() {
        let dir = tempfile::tempdir().unwrap();
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&dst_dir).unwrap();
        let src = dir.path().join("note.txt");
        std::fs::write(&src, "hello").unwrap();

        let state = make_state(dir.path());
        import_one(
            &state,
            src.to_str().unwrap(),
            dst_dir.to_str().unwrap(),
            PlacementMode::Keep,
            Origin::ImportedKept,
        )
        .unwrap();

        assert!(src.exists(), "仅登记不应动原文件");
        assert!(!dst_dir.join("note.txt").exists());
    }

    #[test]
    fn 重复导入返回既有卡片不新建() {
        let dir = tempfile::tempdir().unwrap();
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&dst_dir).unwrap();
        let src = dir.path().join("note.txt");
        std::fs::write(&src, "hello").unwrap();

        let state = make_state(dir.path());
        let first = import_one(
            &state,
            src.to_str().unwrap(),
            dst_dir.to_str().unwrap(),
            PlacementMode::Keep,
            Origin::ImportedKept,
        )
        .unwrap();
        let second = import_one(
            &state,
            src.to_str().unwrap(),
            dst_dir.to_str().unwrap(),
            PlacementMode::Keep,
            Origin::ImportedKept,
        )
        .unwrap();

        assert!(second.existing_card_id.is_some(), "第二次应报已收录");
        assert_eq!(second.existing_card_id, first.card_id);
        let count = state.with_db(db::list_cards).unwrap().len();
        assert_eq!(count, 1, "只应有一张卡");
    }

    #[test]
    fn 源文件不存在时报错且不建卡() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path());
        let err = import_one(
            &state,
            dir.path().join("nope.txt").to_str().unwrap(),
            dir.path().to_str().unwrap(),
            PlacementMode::Keep,
            Origin::ImportedKept,
        )
        .unwrap_err();

        assert!(matches!(err, AppError::SourceMissing(_)));
        assert_eq!(state.with_db(db::list_cards).unwrap().len(), 0);
    }

    #[test]
    fn 未设置目标目录时报错() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("note.txt");
        std::fs::write(&src, "hello").unwrap();
        let state = make_state(dir.path());

        let err = import_one(
            &state,
            src.to_str().unwrap(),
            "",
            PlacementMode::Move,
            Origin::ImportedMoved,
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Invalid(_)), "应提示先设置存储位置");
    }

    #[test]
    fn 目标目录不可写时报错且不建卡() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("note.txt");
        std::fs::write(&src, "hello").unwrap();
        // 在本该是目录的位置放一个文件，让建目录必然失败——这比依赖
        // 只读属性可靠（Windows 上只读目录仍可写入）。
        std::fs::write(dir.path().join("blocker"), "x").unwrap();
        let bad_dir = dir.path().join("blocker").join("such");
        let state = make_state(dir.path());

        let err = import_one(
            &state,
            src.to_str().unwrap(),
            bad_dir.to_str().unwrap(),
            PlacementMode::Move,
            Origin::ImportedMoved,
        )
        .unwrap_err();

        assert!(matches!(err, AppError::Io(_)), "应报文件操作失败");
        assert_eq!(state.with_db(db::list_cards).unwrap().len(), 0, "不应留下卡片");
    }

    #[test]
    fn 文件夹导入整体建卡() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("myfolder");
        std::fs::create_dir_all(src_dir.join("sub")).unwrap();
        std::fs::write(src_dir.join("a.txt"), "a").unwrap();
        std::fs::write(src_dir.join("sub").join("b.txt"), "b").unwrap();
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&dst_dir).unwrap();

        let state = make_state(dir.path());
        let item = import_one(
            &state,
            src_dir.to_str().unwrap(),
            dst_dir.to_str().unwrap(),
            PlacementMode::Move,
            Origin::ImportedMoved,
        )
        .unwrap();

        assert!(item.card_id.is_some());
        let cards = state.with_db(db::list_cards).unwrap();
        assert_eq!(cards.len(), 1, "文件夹整体只建一张卡");
        assert!(dst_dir.join("myfolder").exists(), "文件夹应被移动");
        assert!(cards[0].is_dir, "文件夹卡片应标记 is_dir");
    }

    #[test]
    fn 普通文件的卡片不带_is_dir() {
        let dir = tempfile::tempdir().unwrap();
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&dst_dir).unwrap();
        let src = dir.path().join("note.txt");
        std::fs::write(&src, "hello").unwrap();

        let state = make_state(dir.path());
        import_one(
            &state,
            src.to_str().unwrap(),
            dst_dir.to_str().unwrap(),
            PlacementMode::Keep,
            Origin::ImportedKept,
        )
        .unwrap();

        let cards = state.with_db(db::list_cards).unwrap();
        assert!(!cards[0].is_dir, "普通文件不应标记 is_dir");
    }

    #[test]
    fn 列出文件夹子项() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("f");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("a.txt"), "a").unwrap();
        std::fs::write(root.join("sub").join("b.txt"), "b").unwrap();

        let children = folder_children(root.to_str().unwrap().to_string()).unwrap();
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|c| c.relative_path == "a.txt" && !c.is_dir));
        assert!(children.iter().any(|c| c.relative_path == "sub" && c.is_dir));
    }
}
