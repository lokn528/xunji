//! 缩略图生成。
//!
//! v1 只处理图片类文件。PDF 首页缩略图需要额外的 PDF 渲染依赖，列入后续版本；
//! 非图片类型的卡片允许用户手动粘贴截图。

use std::path::{Path, PathBuf};

use crate::fsops;

/// 缩略图最长边像素。
pub const THUMB_MAX_EDGE: u32 = 480;

/// 为图片文件生成缩略图，返回缓存中的文件路径。
///
/// 非图片文件返回 `None`；图片读取或解码失败时也返回 `None`（不阻塞建卡）。
pub fn generate(source: &str, cache_dir: &str) -> Option<PathBuf> {
    if !fsops::is_image(source) {
        return None;
    }
    let src = Path::new(source);
    if !src.exists() {
        return None;
    }

    let img = image::open(src).ok()?;
    let thumb = img.thumbnail(THUMB_MAX_EDGE, THUMB_MAX_EDGE);

    // 用「源文件路径的哈希」命名，避免同名文件互相覆盖。
    let stem_hash = hash_name(source);
    let out = Path::new(cache_dir).join(format!("{stem_hash}.png"));
    std::fs::create_dir_all(cache_dir).ok()?;
    thumb.save(&out).ok()?;
    Some(out)
}

/// 由源路径推导出稳定且无冲突的缩略图文件名。
pub fn hash_name(source: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    fsops::normalize_path(source).hash(&mut h);
    format!("{:016x}", h.finish())
}

// ----------------------------------------------------------------- 测试

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 非图片不生成缩略图() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("thumbs");
        let pdf = dir.path().join("a.pdf");
        std::fs::write(&pdf, "not really a pdf").unwrap();

        assert!(generate(pdf.to_str().unwrap(), cache.to_str().unwrap()).is_none());
    }

    #[test]
    fn 图片生成缩略图文件() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("thumbs");
        let src = dir.path().join("a.png");

        // 造一个 8x8 的 PNG
        let img = image::RgbImage::from_pixel(8, 8, image::Rgb([200, 30, 30]));
        img.save(&src).unwrap();

        let out = generate(src.to_str().unwrap(), cache.to_str().unwrap());
        assert!(out.is_some(), "应生成缩略图");
        let out = out.unwrap();
        assert!(out.exists());
        assert!(out.to_string_lossy().ends_with(".png"));
    }

    #[test]
    fn 损坏的图片不产生错误只返回空() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("thumbs");
        let src = dir.path().join("broken.png");
        std::fs::write(&src, "not a png at all").unwrap();

        assert!(generate(src.to_str().unwrap(), cache.to_str().unwrap()).is_none());
    }

    #[test]
    fn 同一路径得到同一缩略图名() {
        let a = hash_name(r"C:\docs\a.png");
        let b = hash_name("c:/DOCS/a.png");
        assert_eq!(a, b, "等价路径应得到同一缩略图名");
    }

    #[test]
    fn 不同路径得到不同缩略图名() {
        assert_ne!(hash_name(r"C:\a.png"), hash_name(r"C:\b.png"));
    }
}
