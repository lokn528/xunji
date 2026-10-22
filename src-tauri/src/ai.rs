//! OpenAI 兼容接口的调用与响应解析。
//!
//! 关键设计（见 `plan.md`）：请求在 Rust 侧构造，使"最小内容发送"成为结构性
//! 保证而非前端自觉——`build_request_body` 是全应用唯一能发出内容的地方，
//! 想扩大外发范围必须改这里，而改动会撞上本函数的单元测试。

use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::models::AiSuggestion;

/// 一句话说明的最大长度（字符）。
pub const SUMMARY_MAX_CHARS: usize = 50;

/// 发送给模型的内容片段的最大长度（字符）。
pub const EXCERPT_MAX_CHARS: usize = 500;

/// OpenAI 兼容的对话消息。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChatMessage {
    pub role: &'static str,
    pub content: String,
}

/// OpenAI 兼容的请求体。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f64,
}

/// OpenAI 兼容的响应体（只取用到的字段）。
#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
pub struct ResponseMessage {
    content: String,
}

/// 从 OpenAI 兼容响应中取出文本内容。
pub fn extract_content(json: &serde_json::Value) -> AppResult<String> {
    let resp: ChatResponse = serde_json::from_value(json.clone())
        .map_err(|_| AppError::AiInvalidResponse)?;
    resp.choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .filter(|s| !s.trim().is_empty())
        .ok_or(AppError::AiInvalidResponse)
}

/// 系统提示词——约束输出为可解析的 JSON。
const SYSTEM_PROMPT: &str =
    "你是一个文件整理助手，擅长用一句话说明文件用途。只输出 JSON，不要任何多余文字。";

/// 构造发给模型的请求体。
///
/// `excerpt` 为 `None` 表示不发送任何文件内容（"仅用文件名分析"）。
pub fn build_request_body(
    model: &str,
    file_name: &str,
    file_type: &str,
    excerpt: Option<&str>,
    existing_categories: &[String],
    temperature: f64,
) -> ChatRequest {
    let mut user = format!("文件名：{file_name}\n文件类型：{file_type}");

    if let Some(text) = excerpt {
        let clipped: String = text.chars().take(EXCERPT_MAX_CHARS).collect();
        user.push('\n');
        user.push_str("内容片段：");
        user.push_str(&clipped);
    }

    let cats = if existing_categories.is_empty() {
        "（用户当前没有任何分类）".to_string()
    } else {
        existing_categories.join("、")
    };
    user.push('\n');
    user.push_str("用户已有的分类：");
    user.push_str(&cats);
    user.push('\n');
    user.push_str(&format!(
        "请用一句话（不超过 {SUMMARY_MAX_CHARS} 字）说明这个文件是做什么用的，\
         并从用户已有分类中选一个最贴切的；若都不合适，可以提出一个新分类。\n\
         只返回 JSON，格式为：{{\"summary\": \"一句话说明\", \"category\": \"分类名\"}}"
    ));

    ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system",
                content: SYSTEM_PROMPT.to_string(),
            },
            ChatMessage {
                role: "user",
                content: user,
            },
        ],
        temperature,
    }
}

/// 解析 AI 响应为建议。
///
/// 解析失败一律返回 `AiInvalidResponse`，绝不返回半截内容。
pub fn parse_suggestion(raw: &str) -> AppResult<AiSuggestion> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(AppError::AiInvalidResponse);
    }

    // 模型可能把 JSON 包在 ```json 代码块里，或前后带解释文字。
    let json_text = extract_json_object(raw).ok_or(AppError::AiInvalidResponse)?;

    let value: serde_json::Value =
        serde_json::from_str(&json_text).map_err(|_| AppError::AiInvalidResponse)?;

    let summary = value
        .get("summary")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .ok_or(AppError::AiInvalidResponse)?;

    if summary.is_empty() {
        return Err(AppError::AiInvalidResponse);
    }

    // 截断到上限，保证"一句话说明"不会变成一整段。
    let summary: String = summary.chars().take(SUMMARY_MAX_CHARS).collect();

    let suggested_category = value
        .get("category")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    Ok(AiSuggestion {
        summary,
        suggested_category,
        raw: raw.to_string(),
    })
}

/// 从可能带杂音的响应文本中提取出 JSON 对象。
fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(text[start..=end].to_string())
}

/// 组装完整 URL。
pub fn chat_completions_url(base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{base}/chat/completions")
    }
}

// ----------------------------------------------------------------- 测试

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 仅文件名模式不包含任何文件内容() {
        let body = build_request_body(
            "gpt-4o-mini",
            "高等数学.pdf",
            "pdf",
            None,
            &["数学".to_string()],
            0.3,
        );
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("高等数学.pdf"), "应含文件名");
        assert!(!json.contains("内容片段"), "不得出现内容片段标记");
    }

    #[test]
    fn 允许时请求体包含被裁剪的内容片段() {
        let body = build_request_body(
            "gpt-4o-mini",
            "note.txt",
            "txt",
            Some("这是正文内容"),
            &[],
            0.3,
        );
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("内容片段"), "应含内容片段");
        assert!(json.contains("这是正文内容"));
    }

    #[test]
    fn 内容片段超长时被裁剪() {
        let long = "字".repeat(EXCERPT_MAX_CHARS + 200);
        let body = build_request_body("m", "a.txt", "txt", Some(&long), &[], 0.3);
        let content = &body.messages[1].content;
        let seg = content.split("内容片段：").nth(1).unwrap();
        let seg = seg.split('\n').next().unwrap();
        assert_eq!(seg.chars().count(), EXCERPT_MAX_CHARS, "片段应被裁剪到上限");
    }

    #[test]
    fn 解析正常响应() {
        let raw = r#"{"summary": "这是大一上学期的线性代数教材", "category": "数学"}"#;
        let s = parse_suggestion(raw).unwrap();
        assert_eq!(s.summary, "这是大一上学期的线性代数教材");
        assert_eq!(s.suggested_category.as_deref(), Some("数学"));
    }

    #[test]
    fn 解析包裹在代码块里的响应() {
        let raw = "```json\n{\"summary\": \"期末论文草稿\", \"category\": \"毕设\"}\n```";
        let s = parse_suggestion(raw).unwrap();
        assert_eq!(s.summary, "期末论文草稿");
        assert_eq!(s.suggested_category.as_deref(), Some("毕设"));
    }

    #[test]
    fn 说明超长被截断() {
        let long = "长".repeat(SUMMARY_MAX_CHARS + 30);
        let raw = format!(r#"{{"summary": "{long}"}}"#);
        let s = parse_suggestion(&raw).unwrap();
        assert_eq!(s.summary.chars().count(), SUMMARY_MAX_CHARS);
    }

    #[test]
    fn 分类为空时给出_none() {
        let raw = r#"{"summary": "某个文件"}"#;
        let s = parse_suggestion(raw).unwrap();
        assert_eq!(s.suggested_category, None);
    }

    #[test]
    fn 解析非_json返回错误() {
        assert!(matches!(
            parse_suggestion("抱歉，我无法完成这个请求"),
            Err(AppError::AiInvalidResponse)
        ));
    }

    #[test]
    fn 解析空响应返回错误() {
        assert!(matches!(
            parse_suggestion("   "),
            Err(AppError::AiInvalidResponse)
        ));
    }

    #[test]
    fn 解析缺字段返回错误() {
        assert!(matches!(
            parse_suggestion(r#"{"category": "数学"}"#),
            Err(AppError::AiInvalidResponse)
        ));
        assert!(matches!(
            parse_suggestion(r#"{"summary": "  "}"#),
            Err(AppError::AiInvalidResponse)
        ));
    }

    #[test]
    fn 组装地址兼容带尾斜杠与已含路径() {
        assert_eq!(
            chat_completions_url("https://api.example.com/v1/"),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("https://api.example.com/v1/chat/completions"),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("https://api.example.com"),
            "https://api.example.com/chat/completions"
        );
    }
}
