//! 应用共享状态。
//!
//! 持有一个 SQLite 连接与各项路径配置。所有命令通过 `State<AppState>` 访问。

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// 凭据库服务名。
pub const KEYRING_SERVICE: &str = "xunji";

/// 凭据库中 API Key 的账户名。
pub const KEYRING_ACCOUNT: &str = "ai_api_key";

/// 应用状态。
pub struct AppState {
    /// 数据库连接。用互斥锁串行化访问——单用户桌面应用，并发不是瓶颈，
    /// 换来的是导入事务的确定性。
    pub db: Mutex<Connection>,
    /// 应用数据目录（数据库、缩略图缓存都在其下）。
    pub data_dir: PathBuf,
    /// 启动时从命令行收到的待导入路径（右键「发送到 循迹」会传入文件路径）。
    /// 前端取走一次即清空，避免重复导入。
    pending_imports: Mutex<Vec<String>>,
}

impl AppState {
    /// 新建应用状态。
    pub fn new(db: Connection, data_dir: PathBuf) -> Self {
        AppState {
            db: Mutex::new(db),
            data_dir,
            pending_imports: Mutex::new(Vec::new()),
        }
    }

    /// 数据库文件路径。
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("xunji.db")
    }

    /// 缩略图缓存目录。
    pub fn thumb_dir(&self) -> PathBuf {
        self.data_dir.join("thumbs")
    }

    /// 记下待导入路径。
    pub fn set_pending_imports(&self, paths: Vec<String>) {
        if let Ok(mut guard) = self.pending_imports.lock() {
            *guard = paths;
        }
    }

    /// 取走待导入路径（取走即清空）。
    pub fn take_pending_imports(&self) -> Vec<String> {
        match self.pending_imports.lock() {
            Ok(mut guard) => std::mem::take(&mut *guard),
            Err(_) => Vec::new(),
        }
    }

    /// 取得锁并执行一段数据库操作。
    pub fn with_db<T>(
        &self,
        f: impl FnOnce(&Connection) -> AppResult<T>,
    ) -> AppResult<T> {
        let guard = self
            .db
            .lock()
            .map_err(|_| AppError::Database("数据库锁已中毒".into()))?;
        f(&guard)
    }
}

// ----------------------------------------------------------------- 测试

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 状态可创建并访问数据库() {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::db::open_in_memory().unwrap();
        let state = AppState::new(conn, dir.path().to_path_buf());

        let n = state
            .with_db(|c| {
                let n: i64 = c.query_row("SELECT COUNT(*) FROM cards", [], |r| r.get(0))?;
                Ok(n)
            })
            .unwrap();
        assert_eq!(n, 0);
        assert!(state.db_path().ends_with("xunji.db"));
        assert!(state.thumb_dir().ends_with("thumbs"));
    }

    #[test]
    fn 待导入路径取走即清空() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(
            crate::db::open_in_memory().unwrap(),
            dir.path().to_path_buf(),
        );

        state.set_pending_imports(vec!["C:\\a.txt".to_string()]);
        assert_eq!(state.take_pending_imports().len(), 1);
        assert!(state.take_pending_imports().is_empty(), "取走第二次应为空");
    }
}
