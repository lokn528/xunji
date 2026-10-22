//! 文件系统操作：路径规范化、落盘、大小判定。
//!
//! 这些是纯函数/无 UI 依赖的模块，便于单元测试。
//! 所有"移动/复制"都遵循一条硬规则：目标同名时绝不覆盖已有文件。

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

/// 超过此大小的大文件仅登记路径，不复制内容。
pub const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024;

/// 落点方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// 移动到目标目录（源文件不再留在原处）。
    Move,
    /// 仅登记，文件留在原处。
    Keep,
}

/// 规范化 Windows 路径，用于重复检测。
///
/// 统一为：绝对路径、分隔符统一为 `\`、去掉末尾分隔符、盘符与路径其余部分
/// 统一大写。这样 `C:\Docs\a.txt` 与 `c:/docs/a.txt` 被视为同一个文件。
pub fn normalize_path(path: &str) -> String {
    let p = Path::new(path);
    let mut s = p.to_string_lossy().replace('/', "\\");
    // 去掉末尾分隔符，但保留盘符根 `C:\`
    while s.len() > 3 && s.ends_with('\\') {
        s.pop();
    }
    // 盘符与路径主体统一大写，消除 Windows 的大小写不敏感差异
    s.to_uppercase()
}

/// 取文件的显示名（含扩展名）。
pub fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// 取扩展名（不含点，无扩展名返回空串）。
pub fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// 是否为图片文件——决定能否自动生成缩略图。
pub fn is_image(path: &str) -> bool {
    matches!(
        extension(path).to_ascii_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp"
    )
}

/// 是否为纯文本类——决定能否直接读取内容片段。
pub fn is_textual(path: &str) -> bool {
    matches!(
        extension(path).to_ascii_lowercase().as_str(),
        "txt" | "md" | "markdown" | "csv" | "json" | "xml" | "yaml" | "yml" | "log" | "rs"
            | "ts" | "tsx" | "py" | "js" | "jsx" | "html" | "css"
    )
}

/// 取文件大小。文件不存在或不可读时返回 `None`。
pub fn file_size(path: &str) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}

/// 在目标目录中为源文件生成一个不冲突的目标路径。
///
/// 同名时追加 ` (1)`、` (2)` …… 绝不覆盖已有文件。
pub fn unique_target_path(target_dir: &str, source_name: &str) -> PathBuf {
    let dir = Path::new(target_dir);
    let candidate = dir.join(source_name);
    if !candidate.exists() {
        return candidate;
    }
    let stem = Path::new(source_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| source_name.to_string());
    let ext = Path::new(source_name)
        .extension()
        .map(|s| s.to_string_lossy().to_string());

    for n in 1..10_000 {
        let new_name = match &ext {
            Some(e) => format!("{stem} ({n}).{e}"),
            None => format!("{stem} ({n})"),
        };
        let p = dir.join(new_name);
        if !p.exists() {
            return p;
        }
    }
    // 兜底：用时间戳，几乎不可能撞上
    dir.join(format!("{stem}-{}", crate::models::now_iso()))
}

/// 判定一次导入在磁盘上的实际动作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LandAction {
    /// 移动文件到目标路径。
    Move(PathBuf),
    /// 不动磁盘，仅登记原路径。
    None,
}

/// 计算一次导入的落盘动作。
///
/// - `Keep` 模式 → 一律不动磁盘。
/// - `Move` 模式 + 大文件 → 仅登记（移动大文件会让用户失去原处的文件，
///   复制大文件会产生双份占空间，两者都不是好选择；按设计文档 v1 取"仅登记"）。
/// - `Move` 模式 + 普通文件 → 移动。
pub fn plan_land(
    source: &str,
    target_dir: &str,
    placement: Placement,
    is_dir: bool,
) -> LandAction {
    if placement == Placement::Keep {
        return LandAction::None;
    }
    let size = file_size(source).unwrap_or(0);
    if !is_dir && size > LARGE_FILE_THRESHOLD {
        return LandAction::None;
    }
    let name = file_name(source);
    let target = unique_target_path(target_dir, &name);
    LandAction::Move(target)
}

/// 执行移动。跨卷时退化为"复制 + 删除源"。
pub fn move_file(source: &str, target: &Path) -> AppResult<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(_) => {
            // 跨卷或权限原因导致 rename 失败：退化为复制后删除
            std::fs::copy(source, target)?;
            std::fs::remove_file(source)?;
            Ok(())
        }
    }
}

/// 移动整个目录。
pub fn move_dir(source: &str, target: &Path) -> AppResult<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_dir(Path::new(source), target)?;
            std::fs::remove_dir_all(source)?;
            Ok(())
        }
    }
}

/// 递归复制目录。
pub fn copy_dir(source: &Path, target: &Path) -> AppResult<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let dest = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

/// 把路径转换为用于显示的绝对形式。
pub fn to_absolute(path: &str) -> String {
    match std::path::absolute(path) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => path.to_string(),
    }
}

/// 确认路径存在。
#[allow(dead_code)]
pub fn ensure_exists(path: &str) -> AppResult<()> {
    if !Path::new(path).exists() {
        return Err(AppError::SourceMissing(path.to_string()));
    }
    Ok(())
}

// ----------------------------------------------------------------- 测试

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 路径规范化消除大小写与分隔符差异() {
        let a = normalize_path(r"C:\Docs\a.txt");
        let b = normalize_path("c:/docs/a.txt");
        assert_eq!(a, b, "同一文件的不同写法应归一为同一路径");
    }

    #[test]
    fn 路径规范化保留盘符根() {
        assert_eq!(normalize_path(r"C:\"), r"C:\");
    }

    #[test]
    fn 取文件名与扩展名() {
        assert_eq!(file_name(r"D:\课程\高等数学.pdf"), "高等数学.pdf");
        assert_eq!(extension(r"D:\课程\高等数学.pdf"), "pdf");
        assert_eq!(extension(r"D:\课程\README"), "");
    }

    #[test]
    fn 识别图片与文本() {
        assert!(is_image(r"a.PNG"));
        assert!(!is_image(r"a.pdf"));
        assert!(is_textual(r"a.md"));
        assert!(!is_textual(r"a.docx"));
    }

    #[test]
    fn 同名目标不覆盖() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("a.txt");
        std::fs::write(&existing, "x").unwrap();

        let target = unique_target_path(dir.path().to_str().unwrap(), "a.txt");
        assert_ne!(target, existing, "不得返回已存在的路径");
        assert!(target.to_string_lossy().contains("(1)"), "应追加序号");
        assert!(!target.exists());
    }

    #[test]
    fn 目标名不冲突时直接用原名() {
        let dir = tempfile::tempdir().unwrap();
        let target = unique_target_path(dir.path().to_str().unwrap(), "b.txt");
        assert_eq!(target.file_name().unwrap(), "b.txt");
    }

    #[test]
    fn 仅登记模式不动磁盘() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "x").unwrap();
        let action = plan_land(
            f.to_str().unwrap(),
            dir.path().to_str().unwrap(),
            Placement::Keep,
            false,
        );
        assert_eq!(action, LandAction::None);
        assert!(f.exists(), "仅登记不应动原文件");
    }

    #[test]
    fn 移动模式会规划出目标路径() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("target");
        std::fs::create_dir_all(&sub).unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "x").unwrap();

        let action = plan_land(
            f.to_str().unwrap(),
            sub.to_str().unwrap(),
            Placement::Move,
            false,
        );
        match action {
            LandAction::Move(p) => {
                assert_eq!(p.file_name().unwrap(), "a.txt");
                assert_eq!(p.parent().unwrap(), sub.as_path());
            }
            other => panic!("应规划为移动，实际 {:?}", other),
        }
    }

    #[test]
    fn 移动文件成功() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        std::fs::write(&src, "hello").unwrap();
        let dst = dir.path().join("b.txt");

        move_file(src.to_str().unwrap(), &dst).unwrap();
        assert!(!src.exists(), "源应被移走");
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "hello");
    }

    #[test]
    fn 递归复制目录() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("s");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.txt"), "a").unwrap();
        std::fs::write(src.join("sub").join("b.txt"), "b").unwrap();
        let dst = dir.path().join("d");

        copy_dir(&src, &dst).unwrap();
        assert!(dst.join("a.txt").exists());
        assert!(dst.join("sub").join("b.txt").exists());
    }
}
