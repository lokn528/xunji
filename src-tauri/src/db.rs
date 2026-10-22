//! SQLite 数据访问层。
//!
//! 所有 SQL 集中在此模块，命令层只做编排。迁移用 `user_version` PRAGMA 管理，
//! 语句以 `include_str!` 嵌入，保证编译期与应用版本一致。

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, AppResult};
use crate::models::{now_iso, Card, Category, Origin, PlacementMode, Settings};

/// version 0 的建表与索引语句。
const SCHEMA_V0: &str = include_str!("../migrations/000_initial.sql");

/// version 1：给卡片加 is_dir 字段。
const SCHEMA_V1: &str = include_str!("../migrations/001_add_is_dir.sql");

/// 迁移步骤表。
///
/// 约定：`user_version = N` 表示前 N 个迁移已应用。因此第 i 个迁移的 `version`
/// 是 i，应用后 `user_version` 变成 i + 1。SQLite 的 `user_version` 默认是 0，
/// 正好对应"一个迁移都还没应用"。
const MIGRATIONS: &[(i64, &str)] = &[(0, SCHEMA_V0), (1, SCHEMA_V1)];

/// 期望的 schema 版本，等于已应用迁移的条数。
const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// 打开数据库并确保 schema 为最新。
pub fn open(path: &Path) -> AppResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    migrate(&conn)?;
    Ok(conn)
}

/// 打开内存库，供单元测试使用。
#[allow(dead_code)]
pub fn open_in_memory() -> AppResult<Connection> {
    let conn = Connection::open_in_memory()?;
    migrate(&conn)?;
    Ok(conn)
}

/// 按 `user_version` 逐版本迁移。
fn migrate(conn: &Connection) -> AppResult<()> {
    let mut current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current > SCHEMA_VERSION {
        return Err(AppError::Database(format!(
            "数据库版本 {current} 高于当前程序支持的 {SCHEMA_VERSION}"
        )));
    }
    for (version, sql) in MIGRATIONS {
        if *version >= current {
            conn.execute_batch(sql)?;
            current = *version + 1;
            conn.pragma_update(None, "user_version", current)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- cards

/// 查询全部卡片：置顶优先，其后按创建时间倒序。
pub fn list_cards(conn: &Connection) -> AppResult<Vec<Card>> {
    let mut stmt = conn.prepare(
        "SELECT id, display_name, summary, file_path, thumb_path, category_id,
                origin, ai_generated, needs_summary, missing, pinned,
                created_at, updated_at, is_dir
           FROM cards
          ORDER BY pinned DESC, created_at DESC, id DESC",
    )?;
    let rows = stmt.query_map([], map_card)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// 按 id 查询一张卡片。
pub fn get_card(conn: &Connection, id: &str) -> AppResult<Option<Card>> {
    let mut stmt = conn.prepare(
        "SELECT id, display_name, summary, file_path, thumb_path, category_id,
                origin, ai_generated, needs_summary, missing, pinned,
                created_at, updated_at, is_dir
           FROM cards WHERE id = ?1",
    )?;
    Ok(stmt.query_row(params![id], map_card).optional()?)
}

/// 按 file_path 查询卡片——重复检测用。
pub fn find_card_by_path(conn: &Connection, file_path: &str) -> AppResult<Option<Card>> {
    let mut stmt = conn.prepare(
        "SELECT id, display_name, summary, file_path, thumb_path, category_id,
                origin, ai_generated, needs_summary, missing, pinned,
                created_at, updated_at, is_dir
           FROM cards WHERE file_path = ?1",
    )?;
    Ok(stmt.query_row(params![file_path], map_card).optional()?)
}

/// 插入一张卡片。`file_path` 唯一约束冲突时返回 `AlreadyExists`。
pub fn insert_card(conn: &Connection, card: &Card) -> AppResult<()> {
    let res = conn.execute(
        "INSERT INTO cards
           (id, display_name, summary, file_path, thumb_path, category_id, origin,
            ai_generated, needs_summary, missing, pinned, created_at, updated_at, is_dir)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            card.id,
            card.display_name,
            card.summary,
            card.file_path,
            card.thumb_path,
            card.category_id,
            card.origin.as_str(),
            card.ai_generated as i64,
            card.needs_summary as i64,
            card.missing as i64,
            card.pinned as i64,
            card.created_at,
            card.updated_at,
            card.is_dir as i64,
        ],
    );
    match res {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(AppError::AlreadyExists(card.id.clone()))
        }
        Err(e) => Err(e.into()),
    }
}

/// 更新说明。写入非空说明时同步清除 `needs_summary`。
pub fn update_summary(
    conn: &Connection,
    id: &str,
    summary: &str,
    ai_generated: bool,
) -> AppResult<()> {
    let trimmed = summary.trim();
    let needs_summary = trimmed.is_empty();
    let changed = conn.execute(
        "UPDATE cards
            SET summary = ?2,
                ai_generated = ?3,
                needs_summary = ?4,
                updated_at = ?5
          WHERE id = ?1",
        params![id, trimmed, ai_generated as i64, needs_summary as i64, now_iso()],
    )?;
    if changed == 0 {
        return Err(AppError::Invalid("卡片不存在".into()));
    }
    Ok(())
}

/// 更新显示名。
pub fn update_display_name(conn: &Connection, id: &str, name: &str) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE cards SET display_name = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, name, now_iso()],
    )?;
    if changed == 0 {
        return Err(AppError::Invalid("卡片不存在".into()));
    }
    Ok(())
}

/// 设置分类。分类不存在时按名创建。传入空名表示取消分类。
pub fn set_category(conn: &Connection, card_id: &str, category_name: &str) -> AppResult<String> {
    let name = category_name.trim();
    if name.is_empty() {
        conn.execute(
            "UPDATE cards SET category_id = NULL, updated_at = ?2 WHERE id = ?1",
            params![card_id, now_iso()],
        )?;
        return Ok(String::new());
    }
    let cat_id = ensure_category(conn, name)?;
    conn.execute(
        "UPDATE cards SET category_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![card_id, cat_id, now_iso()],
    )?;
    Ok(cat_id)
}

/// 设置置顶状态。
pub fn set_pinned(conn: &Connection, card_id: &str, pinned: bool) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE cards SET pinned = ?2, updated_at = ?3 WHERE id = ?1",
        params![card_id, pinned as i64, now_iso()],
    )?;
    if changed == 0 {
        return Err(AppError::Invalid("卡片不存在".into()));
    }
    Ok(())
}

/// 重新定位：把卡片绑定到新的文件路径。
///
/// 保留卡片 id、说明与分类（constitution 原则 V）。若新路径已被别的卡片占用则报错。
pub fn relink_card(conn: &Connection, card_id: &str, new_path: &str) -> AppResult<()> {
    if let Some(other) = find_card_by_path(conn, new_path)? {
        if other.id != card_id {
            return Err(AppError::Invalid("该文件已被其他卡片收录".into()));
        }
    }
    let changed = conn.execute(
        "UPDATE cards SET file_path = ?2, missing = 0, updated_at = ?3 WHERE id = ?1",
        params![card_id, new_path, now_iso()],
    )?;
    if changed == 0 {
        return Err(AppError::Invalid("卡片不存在".into()));
    }
    Ok(())
}

/// 回写源文件存在性标记。
pub fn set_missing(conn: &Connection, card_id: &str, missing: bool) -> AppResult<()> {
    conn.execute(
        "UPDATE cards SET missing = ?2 WHERE id = ?1",
        params![card_id, missing as i64],
    )?;
    Ok(())
}

/// 删除卡片。文件本体不受影响（v1 不做文件删除）。
pub fn delete_card(conn: &Connection, card_id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM cards WHERE id = ?1", params![card_id])?;
    Ok(())
}

// ------------------------------------------------------------ categories

/// 查询全部分类，附各分类卡片数。
pub fn list_categories(conn: &Connection) -> AppResult<Vec<Category>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, c.created_at, c.sort_order,
                (SELECT COUNT(*) FROM cards WHERE cards.category_id = c.id)
           FROM categories c
          ORDER BY c.sort_order ASC, c.name ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Category {
            id: r.get(0)?,
            name: r.get(1)?,
            created_at: r.get(2)?,
            sort_order: r.get(3)?,
            card_count: r.get(4)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// 按名取分类 id；不存在则创建。
pub fn ensure_category(conn: &Connection, name: &str) -> AppResult<String> {
    let name = name.trim();
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM categories WHERE name = ?1",
            params![name],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO categories (id, name, created_at, sort_order) VALUES (?1, ?2, ?3, 0)",
        params![id, name, now_iso()],
    )?;
    Ok(id)
}

/// 创建分类。重名时报错。
pub fn create_category(conn: &Connection, name: &str) -> AppResult<Category> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("分类名不能为空".into()));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = now_iso();
    let res = conn.execute(
        "INSERT INTO categories (id, name, created_at, sort_order) VALUES (?1, ?2, ?3, 0)",
        params![id, name, created_at],
    );
    match res {
        Ok(_) => Ok(Category {
            id,
            name: name.to_string(),
            created_at,
            sort_order: 0,
            card_count: 0,
        }),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(AppError::Invalid("该分类已存在".into()))
        }
        Err(e) => Err(e.into()),
    }
}

/// 删除分类。其下卡片保留，category_id 置空。
pub fn delete_category(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM categories WHERE id = ?1", params![id])?;
    Ok(())
}

// -------------------------------------------------------------- settings

/// 读取单个设置值。
pub fn get_setting(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| {
            r.get(0)
        })
        .optional()?)
}

/// 写入单个设置值。
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// 读取完整设置，缺失项用默认值补齐。
pub fn load_settings(conn: &Connection) -> AppResult<Settings> {
    let mut s = Settings::default();
    if let Some(v) = get_setting(conn, "storage.default_dir")? {
        s.default_dir = v;
    }
    if let Some(v) = get_setting(conn, "storage.placement_mode")? {
        s.placement_mode = PlacementMode::parse(&v);
    }
    if let Some(v) = get_setting(conn, "ai.provider")? {
        s.ai_provider = v;
    }
    if let Some(v) = get_setting(conn, "ai.base_url")? {
        s.ai_base_url = v;
    }
    if let Some(v) = get_setting(conn, "ai.model")? {
        s.ai_model = v;
    }
    if let Some(v) = get_setting(conn, "ai.temperature")? {
        if let Ok(t) = v.parse::<f64>() {
            s.ai_temperature = t;
        }
    }
    if let Some(v) = get_setting(conn, "ai.filename_only")? {
        s.ai_filename_only = v != "0";
    }
    if let Some(v) = get_setting(conn, "thumb.cache_dir")? {
        s.thumb_cache_dir = v;
    }
    if let Some(v) = get_setting(conn, "app.autostart")? {
        s.autostart = v != "0";
    }
    Ok(s)
}

/// 保存设置（非敏感部分）。
pub fn save_settings(conn: &Connection, s: &Settings) -> AppResult<()> {
    set_setting(conn, "storage.default_dir", &s.default_dir)?;
    set_setting(conn, "storage.placement_mode", s.placement_mode.as_str())?;
    set_setting(conn, "ai.provider", &s.ai_provider)?;
    set_setting(conn, "ai.base_url", &s.ai_base_url)?;
    set_setting(conn, "ai.model", &s.ai_model)?;
    set_setting(conn, "ai.temperature", &s.ai_temperature.to_string())?;
    set_setting(
        conn,
        "ai.filename_only",
        if s.ai_filename_only { "1" } else { "0" },
    )?;
    set_setting(conn, "thumb.cache_dir", &s.thumb_cache_dir)?;
    set_setting(
        conn,
        "app.autostart",
        if s.autostart { "1" } else { "0" },
    )?;
    Ok(())
}

// ----------------------------------------------------------------- 内部

/// 把一行查询结果映射为 `Card`。
fn map_card(r: &rusqlite::Row<'_>) -> rusqlite::Result<Card> {
    let origin: String = r.get(6)?;
    let is_dir: i64 = r.get(13)?;
    Ok(Card {
        id: r.get(0)?,
        display_name: r.get(1)?,
        summary: r.get(2)?,
        file_path: r.get(3)?,
        thumb_path: r.get(4)?,
        category_id: r.get(5)?,
        origin: Origin::parse(&origin),
        ai_generated: r.get::<_, i64>(7)? != 0,
        needs_summary: r.get::<_, i64>(8)? != 0,
        missing: r.get::<_, i64>(9)? != 0,
        is_dir: is_dir != 0,
        pinned: r.get::<_, i64>(10)? != 0,
        created_at: r.get(11)?,
        updated_at: r.get(12)?,
    })
}

// ----------------------------------------------------------------- 测试

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_card(path: &str) -> Card {
        Card {
            id: uuid::Uuid::new_v4().to_string(),
            display_name: "test.txt".into(),
            summary: String::new(),
            file_path: path.into(),
            thumb_path: None,
            category_id: None,
            origin: Origin::ImportedMoved,
            ai_generated: false,
            needs_summary: true,
            missing: false,
            is_dir: false,
            pinned: false,
            created_at: now_iso(),
            updated_at: now_iso(),
        }
    }

    #[test]
    fn 建表后三张表与索引都存在() {
        let conn = open_in_memory().unwrap();
        for table in ["cards", "categories", "settings"] {
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    params![table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "{table} 表应存在");
        }
        let idx: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_cards_created_at'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx, 1, "时间线索引应存在");
    }

    #[test]
    fn 卡片增删改查() {
        let conn = open_in_memory().unwrap();
        let card = sample_card(r"C:\docs\a.txt");
        insert_card(&conn, &card).unwrap();

        let got = get_card(&conn, &card.id).unwrap().unwrap();
        assert_eq!(got.file_path, card.file_path);
        assert!(got.needs_summary, "空说明应标记待写");

        update_summary(&conn, &card.id, "这是我的作业说明", true).unwrap();
        let got = get_card(&conn, &card.id).unwrap().unwrap();
        assert_eq!(got.summary, "这是我的作业说明");
        assert!(!got.needs_summary, "写入说明后应清除待写标记");
        assert!(got.ai_generated);

        set_pinned(&conn, &card.id, true).unwrap();
        assert!(get_card(&conn, &card.id).unwrap().unwrap().pinned);

        delete_card(&conn, &card.id).unwrap();
        assert!(get_card(&conn, &card.id).unwrap().is_none());
    }

    #[test]
    fn 同一路径不能建两张卡() {
        let conn = open_in_memory().unwrap();
        let a = sample_card(r"C:\docs\a.txt");
        insert_card(&conn, &a).unwrap();

        let mut b = sample_card(r"C:\docs\a.txt");
        b.id = uuid::Uuid::new_v4().to_string();
        let err = insert_card(&conn, &b).unwrap_err();
        assert!(matches!(err, AppError::AlreadyExists(_)), "应报重复");

        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM cards", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "只应留下一张卡");
    }

    #[test]
    fn 置顶卡片排在最前() {
        let conn = open_in_memory().unwrap();
        let old = sample_card(r"C:\docs\old.txt");
        let new = sample_card(r"C:\docs\new.txt");
        insert_card(&conn, &old).unwrap();
        insert_card(&conn, &new).unwrap();
        conn.execute(
            "UPDATE cards SET created_at = '2020-01-01T00:00:00.000Z' WHERE id = ?1",
            params![old.id],
        )
        .unwrap();
        set_pinned(&conn, &old.id, true).unwrap();

        let list = list_cards(&conn).unwrap();
        assert_eq!(list[0].id, old.id, "置顶卡片应排最前");
    }

    #[test]
    fn 删除分类后卡片保留且分类置空() {
        let conn = open_in_memory().unwrap();
        let card = sample_card(r"C:\docs\a.txt");
        insert_card(&conn, &card).unwrap();
        let cat_id = set_category(&conn, &card.id, "数学").unwrap();
        assert_eq!(
            get_card(&conn, &card.id).unwrap().unwrap().category_id,
            Some(cat_id.clone())
        );

        delete_category(&conn, &cat_id).unwrap();
        let got = get_card(&conn, &card.id).unwrap().unwrap();
        assert_eq!(got.id, card.id, "卡片不应被删除");
        assert_eq!(got.category_id, None, "分类应置空");
    }

    #[test]
    fn 重名分类被拒() {
        let conn = open_in_memory().unwrap();
        create_category(&conn, "数学").unwrap();
        let err = create_category(&conn, "数学").unwrap_err();
        assert!(matches!(err, AppError::Invalid(_)), "重名应报错");
    }

    #[test]
    fn 重新定位保留说明与分类() {
        let conn = open_in_memory().unwrap();
        let card = sample_card(r"C:\docs\a.txt");
        insert_card(&conn, &card).unwrap();
        update_summary(&conn, &card.id, "保留这句说明", false).unwrap();
        let cat = set_category(&conn, &card.id, "毕设").unwrap();

        relink_card(&conn, &card.id, r"C:\new\a.txt").unwrap();
        let got = get_card(&conn, &card.id).unwrap().unwrap();
        assert_eq!(got.file_path, r"C:\new\a.txt");
        assert_eq!(got.summary, "保留这句说明", "说明应保留");
        assert_eq!(got.category_id, Some(cat), "分类应保留");
        assert!(!got.missing, "重新定位后应清除丢失标记");
    }

    #[test]
    fn 重新定位到已被占用的路径时报错() {
        let conn = open_in_memory().unwrap();
        let a = sample_card(r"C:\docs\a.txt");
        let b = sample_card(r"C:\docs\b.txt");
        insert_card(&conn, &a).unwrap();
        insert_card(&conn, &b).unwrap();

        let err = relink_card(&conn, &a.id, r"C:\docs\b.txt").unwrap_err();
        assert!(matches!(err, AppError::Invalid(_)), "应拒绝占用已收录路径");
    }

    #[test]
    fn 设置可持久化并补齐默认值() {
        let conn = open_in_memory().unwrap();
        let mut s = load_settings(&conn).unwrap();
        assert!(s.ai_filename_only, "默认应为仅用文件名分析");
        assert_eq!(s.placement_mode, PlacementMode::Move);

        s.default_dir = r"D:\循迹库".into();
        s.ai_model = "gpt-4o-mini".into();
        save_settings(&conn, &s).unwrap();

        let back = load_settings(&conn).unwrap();
        assert_eq!(back.default_dir, r"D:\循迹库");
        assert_eq!(back.ai_model, "gpt-4o-mini");
    }

    #[test]
    fn 保存设置不会写入_api_key() {
        let conn = open_in_memory().unwrap();
        let s = Settings::default();
        save_settings(&conn, &s).unwrap();

        let leaked: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE value LIKE 'sk-%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(leaked, 0, "设置表不得出现 key 形态的值");
    }

    #[test]
    fn 迁移后版本号与预期一致() {
        let conn = open_in_memory().unwrap();
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, SCHEMA_VERSION, "user_version 应等于最新迁移版本");
    }

    #[test]
    fn is_dir_列存在且默认_false() {
        let conn = open_in_memory().unwrap();
        let card = sample_card(r"C:\docs\a.txt");
        insert_card(&conn, &card).unwrap();
        let got = get_card(&conn, &card.id).unwrap().unwrap();
        assert!(!got.is_dir, "默认应为普通文件");
    }

    #[test]
    fn 旧版本数据库会被自动升级() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.db");

        // 造一个只有 v0 schema 的旧库，user_version = 1 表示已应用 1 个迁移
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(SCHEMA_V0).unwrap();
            conn.pragma_update(None, "user_version", 1i64).unwrap();
        }

        // 打开时应自动补上 is_dir 列
        let conn = open(&path).unwrap();
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, SCHEMA_VERSION, "旧库应被升级到最新版本");

        let card = sample_card(r"C:\docs\a.txt");
        insert_card(&conn, &card).unwrap();
        let got = get_card(&conn, &card.id).unwrap().unwrap();
        assert!(!got.is_dir, "升级后的卡片应能正常读写");
    }
}
