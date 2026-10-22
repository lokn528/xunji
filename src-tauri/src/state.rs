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
}

impl AppState {
    /// 数据库文件路径。
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("xunji.db")
    }

    /// 缩略图缓存目录。
    pub fn thumb_dir(&self) -> PathBuf {
        self.data_dir.join("thumbs")
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
        let state = AppState {
            db: Mutex::new(conn),
            data_dir: dir.path().to_path_buf(),
        };

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
}
