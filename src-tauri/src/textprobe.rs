//! 可提取文本片段的探测。
//!
//! v1 只读纯文本类文件的前若干字符。PDF 正文提取需要额外依赖，列入后续版本；
//! 图片类一律不传内容（constitution 原则 IV）。

use crate::fsops;

/// 读取文本片段的最大字节数。
pub const MAX_EXCERPT_BYTES: usize = 4096;

/// 提取文件的文本片段。
///
/// 返回 `None` 表示该文件类型不传内容，或读取失败。
pub fn excerpt(path: &str) -> Option<String> {
    if !fsops::is_textual(path) {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let slice = &bytes[..bytes.len().min(MAX_EXCERPT_BYTES)];
    // 截断可能落在多字节字符中间，用 from_utf8_lossy 兜底
    let text = String::from_utf8_lossy(slice);
    let cleaned: String = text
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

// ----------------------------------------------------------------- 测试

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 文本文件可提取片段() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "这是我的课程笔记。").unwrap();

        let e = excerpt(f.to_str().unwrap()).unwrap();
        assert!(e.contains("课程笔记"));
    }

    #[test]
    fn 图片不提取内容() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.png");
        std::fs::write(&f, "binary-ish").unwrap();

        assert!(excerpt(f.to_str().unwrap()).is_none(), "图片不得提取内容");
    }

    #[test]
    fn pdf不提取内容() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.pdf");
        std::fs::write(&f, "%PDF-1.4 ...").unwrap();

        assert!(excerpt(f.to_str().unwrap()).is_none(), "v1 不处理 PDF 正文");
    }

    #[test]
    fn 空文件返回空() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "   \n  ").unwrap();

        assert!(excerpt(f.to_str().unwrap()).is_none());
    }

    #[test]
    fn 超长文件被截断() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        let long = "字".repeat(MAX_EXCERPT_BYTES);
        std::fs::write(&f, &long).unwrap();

        let e = excerpt(f.to_str().unwrap()).unwrap();
        assert!(e.chars().count() <= MAX_EXCERPT_BYTES, "片段应被截断");
    }
}
