//! 统一错误类型。
//!
//! 每个变体都对应前端需要区分处理的一种情况，序列化后前端可据 `code` 分支，
//! `message` 直接用于展示。

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// 目标位置磁盘空间不足。
    #[error("目标位置磁盘空间不足")]
    DiskFull,

    /// 没有写入权限。
    #[error("没有写入权限：{0}")]
    PermissionDenied(String),

    /// 源文件已不存在。
    #[error("源文件已不存在：{0}")]
    SourceMissing(String),

    /// 该文件已被收录，返回既有卡片 id。
    #[error("该文件已收录")]
    AlreadyExists(String),

    /// AI 未配置或不可用。
    #[error("AI 不可用，可手动填写")]
    AiUnavailable,

    /// AI 请求失败（网络、超时、非 2xx）。
    #[error("AI 请求失败：{0}")]
    AiRequestFailed(String),

    /// AI 返回内容无法解析。
    #[error("AI 返回内容无法解析，请手动填写")]
    AiInvalidResponse,

    /// 数据库错误。
    #[error("数据库错误：{0}")]
    Database(String),

    /// 通用文件操作错误。
    #[error("文件操作失败：{0}")]
    Io(String),

    /// 参数或状态不合法。
    #[error("{0}")]
    Invalid(String),
}

impl AppError {
    /// 供前端分支判断的稳定错误码。
    pub fn code(&self) -> &'static str {
        match self {
            AppError::DiskFull => "DISK_FULL",
            AppError::PermissionDenied(_) => "PERMISSION_DENIED",
            AppError::SourceMissing(_) => "SOURCE_MISSING",
            AppError::AlreadyExists(_) => "ALREADY_EXISTS",
            AppError::AiUnavailable => "AI_UNAVAILABLE",
            AppError::AiRequestFailed(_) => "AI_REQUEST_FAILED",
            AppError::AiInvalidResponse => "AI_INVALID_RESPONSE",
            AppError::Database(_) => "DATABASE",
            AppError::Io(_) => "IO",
            AppError::Invalid(_) => "INVALID",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        // 磁盘满与权限拒绝是导入流程中最需要区分的两类失败。
        match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                AppError::PermissionDenied(e.to_string())
            }
            std::io::ErrorKind::StorageFull => AppError::DiskFull,
            _ => AppError::Io(e.to_string()),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
