//! 模型接口的请求构造与响应解析。
//!
//! 关键设计（见 `plan.md`）：请求在 Rust 侧构造，使“最小内容发送”成为结构性
//! 保证而非前端自觉——`build_request_body` 是全应用唯一能发出文件内容的地方，
//! 想扩大外发范围必须改这里，而改动会撞上本函数的单元测试。

use serde::Deserialize;
use serde_json::json;

use crate::error::{AppError, AppResult};
use crate::models::{AiProtocol, AiSuggestion, AiUsage};

/// 一句话说明的最大长度（字符）。
pub const SUMMARY_MAX_CHARS: usize = 50;

/// 发送给模型的内容片段的最大长度（字符）。
pub const EXCERPT_MAX_CHARS: usize = 500;

/// OpenAI 兼容的响应体（只取用到的字段）。
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

/// OpenAI Responses 的响应体。
#[derive(Debug, Deserialize)]
struct ResponsesResponse {
    #[serde(default)]
    output_text: Option<String>,
    #[serde(default)]
    output: Vec<ResponsesOutput>,
}

#[derive(Debug, Deserialize)]
struct ResponsesOutput {
    #[serde(default)]
    content: Vec<ResponsesContent>,
}

#[derive(Debug, Deserialize)]
struct ResponsesContent {
    #[serde(default)]
    text: Option<String>,
    #[serde(default, rename = "type")]
    content_type: Option<String>,
}

/// Anthropic Messages 的响应体。
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(default)]
    text: Option<String>,
    #[serde(default, rename = "type")]
    content_type: Option<String>,
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.trim().is_empty())
}

/// 按协议从响应中取出文本内容。
pub fn extract_content(protocol: AiProtocol, json: &serde_json::Value) -> AppResult<String> {
    match protocol {
        AiProtocol::OpenAiChat => {
            let resp: ChatResponse =
                serde_json::from_value(json.clone()).map_err(|_| AppError::AiInvalidResponse)?;
            resp.choices
                .into_iter()
                .next()
                .map(|choice| choice.message.content)
                .filter(|text| !text.trim().is_empty())
                .ok_or(AppError::AiInvalidResponse)
        }
        AiProtocol::OpenAiResponses => {
            let resp: ResponsesResponse =
                serde_json::from_value(json.clone()).map_err(|_| AppError::AiInvalidResponse)?;
            let direct = non_empty(resp.output_text);
            let nested = resp
                .output
                .into_iter()
                .flat_map(|item| item.content)
                .filter(|part| {
                    part.content_type
                        .as_deref()
                        .is_none_or(|kind| kind == "output_text")
                })
                .filter_map(|part| part.text)
                .find(|text| !text.trim().is_empty());
            direct.or(nested).ok_or(AppError::AiInvalidResponse)
        }
        AiProtocol::AnthropicMessages => {
            let resp: AnthropicResponse =
                serde_json::from_value(json.clone()).map_err(|_| AppError::AiInvalidResponse)?;
            resp.content
                .into_iter()
                .filter(|part| {
                    part.content_type
                        .as_deref()
                        .is_none_or(|kind| kind == "text")
                })
                .filter_map(|part| part.text)
                .find(|text| !text.trim().is_empty())
                .ok_or(AppError::AiInvalidResponse)
        }
    }
}

fn token_count(usage: &serde_json::Value, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| usage.get(*key).and_then(serde_json::Value::as_u64))
}

fn nested_token_count(usage: &serde_json::Value, outer: &str, inner: &str) -> Option<u64> {
    usage
        .get(outer)
        .and_then(|details| details.get(inner))
        .and_then(serde_json::Value::as_u64)
}

fn positive(value: Option<u64>) -> Option<u64> {
    value.filter(|tokens| *tokens > 0)
}

/// 从三种协议的响应中归一化 token 用量。
///
/// OpenAI Chat 使用 prompt/completion 命名，Responses 与 Anthropic 使用
/// input/output 命名。部分兼容接口缺少 total 或 details，因此这里按可选字段处理。
pub fn extract_usage(protocol: AiProtocol, json: &serde_json::Value) -> Option<AiUsage> {
    let usage = json.get("usage")?;

    let (input, output, total, reasoning, cached) = match protocol {
        AiProtocol::OpenAiChat => (
            token_count(usage, &["prompt_tokens", "input_tokens"]),
            token_count(usage, &["completion_tokens", "output_tokens"]),
            token_count(usage, &["total_tokens"]),
            nested_token_count(usage, "completion_tokens_details", "reasoning_tokens")
                .or_else(|| nested_token_count(usage, "output_tokens_details", "reasoning_tokens"))
                .or_else(|| token_count(usage, &["reasoning_tokens"])),
            nested_token_count(usage, "prompt_tokens_details", "cached_tokens")
                .or_else(|| token_count(usage, &["prompt_cache_hit_tokens", "cached_tokens"])),
        ),
        AiProtocol::OpenAiResponses => (
            token_count(usage, &["input_tokens", "prompt_tokens"]),
            token_count(usage, &["output_tokens", "completion_tokens"]),
            token_count(usage, &["total_tokens"]),
            nested_token_count(usage, "output_tokens_details", "reasoning_tokens")
                .or_else(|| {
                    nested_token_count(usage, "completion_tokens_details", "reasoning_tokens")
                })
                .or_else(|| token_count(usage, &["reasoning_tokens"])),
            nested_token_count(usage, "input_tokens_details", "cached_tokens")
                .or_else(|| nested_token_count(usage, "prompt_tokens_details", "cached_tokens"))
                .or_else(|| token_count(usage, &["cached_tokens"])),
        ),
        AiProtocol::AnthropicMessages => {
            // Anthropic 的 input_tokens 不含缓存读写，真实输入量需要三者相加。
            let uncached = token_count(usage, &["input_tokens", "prompt_tokens"]);
            let cache_creation = token_count(usage, &["cache_creation_input_tokens"]);
            let cache_read = token_count(usage, &["cache_read_input_tokens"]);
            let input = if uncached.is_none() && cache_creation.is_none() && cache_read.is_none() {
                None
            } else {
                Some(
                    uncached
                        .unwrap_or(0)
                        .saturating_add(cache_creation.unwrap_or(0))
                        .saturating_add(cache_read.unwrap_or(0)),
                )
            };
            (
                input,
                token_count(usage, &["output_tokens", "completion_tokens"]),
                token_count(usage, &["total_tokens"]),
                nested_token_count(usage, "output_tokens_details", "reasoning_tokens")
                    .or_else(|| token_count(usage, &["reasoning_tokens"])),
                cache_read,
            )
        }
    };

    if input.is_none() && output.is_none() && total.is_none() {
        return None;
    }

    let input_tokens = input.unwrap_or(0);
    let output_tokens = output.unwrap_or(0);
    Some(AiUsage {
        input_tokens,
        output_tokens,
        total_tokens: total.unwrap_or_else(|| input_tokens.saturating_add(output_tokens)),
        reasoning_tokens: positive(reasoning),
        cached_tokens: positive(cached),
    })
}

/// 系统提示词——约束输出为可解析的 JSON。
const SYSTEM_PROMPT: &str =
    "你是一个文件整理助手，擅长用一句话说明文件用途。只输出 JSON，不要任何多余文字。";

fn build_user_prompt(
    file_name: &str,
    file_type: &str,
    excerpt: Option<&str>,
    existing_categories: &[String],
) -> String {
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
         并从用户已有分类中选 1 到 5 个最贴切的；若都不合适，可以提出新分类。\n\
         只返回 JSON，格式为：{{\"summary\": \"一句话说明\", \"categories\": [\"分类名\"]}}"
    ));
    user
}

fn build_body_from_prompt(
    protocol: AiProtocol,
    model: &str,
    system: &str,
    user: &str,
    temperature: f64,
) -> serde_json::Value {
    match protocol {
        AiProtocol::OpenAiChat => json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ],
            "temperature": temperature
        }),
        AiProtocol::OpenAiResponses => json!({
            "model": model,
            "instructions": system,
            "input": user,
            "temperature": temperature
        }),
        AiProtocol::AnthropicMessages => json!({
            "model": model,
            "system": system,
            "messages": [
                { "role": "user", "content": user }
            ],
            "max_tokens": 512,
            "temperature": temperature
        }),
    }
}

/// 构造发给模型的请求体。
///
/// `excerpt` 为 `None` 表示不发送任何文件内容（“仅用文件名分析”）。
pub fn build_request_body(
    protocol: AiProtocol,
    model: &str,
    file_name: &str,
    file_type: &str,
    excerpt: Option<&str>,
    existing_categories: &[String],
    temperature: f64,
) -> serde_json::Value {
    let user = build_user_prompt(file_name, file_type, excerpt, existing_categories);
    build_body_from_prompt(protocol, model, SYSTEM_PROMPT, &user, temperature)
}

/// 构造连通性检查的最小请求体。
pub fn build_probe_body(protocol: AiProtocol, model: &str) -> serde_json::Value {
    build_body_from_prompt(
        protocol,
        model,
        "你是接口连通性检查助手。",
        "只回复 pong",
        0.0,
    )
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

    // 截断到上限，保证“一句话说明”不会变成一整段。
    let summary: String = summary.chars().take(SUMMARY_MAX_CHARS).collect();

    let mut suggested_categories = Vec::new();
    if let Some(items) = value.get("categories").and_then(|v| v.as_array()) {
        for item in items.iter().filter_map(|v| v.as_str()) {
            let name = item.trim();
            if !name.is_empty()
                && !suggested_categories.iter().any(|existing| existing == name)
            {
                suggested_categories.push(name.to_string());
            }
            if suggested_categories.len() == 5 {
                break;
            }
        }
    } else if let Some(name) = value
        .get("category")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        // 兼容仍返回旧单分类格式的模型。
        suggested_categories.push(name.to_string());
    }

    Ok(AiSuggestion {
        summary,
        suggested_categories,
        raw: raw.to_string(),
        usage: None,
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

fn endpoint_url(base_url: &str, endpoint: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if base.ends_with(endpoint) {
        base.to_string()
    } else {
        format!("{base}{endpoint}")
    }
}

/// 按协议组装完整 API URL。
pub fn api_url(protocol: AiProtocol, base_url: &str) -> String {
    let endpoint = match protocol {
        AiProtocol::OpenAiChat => "/chat/completions",
        AiProtocol::OpenAiResponses => "/responses",
        AiProtocol::AnthropicMessages => "/messages",
    };
    endpoint_url(base_url, endpoint)
}

// ----------------------------------------------------------------- 测试

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 仅文件名模式不包含任何文件内容() {
        let body = build_request_body(
            AiProtocol::OpenAiChat,
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
            AiProtocol::OpenAiChat,
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
        let body = build_request_body(
            AiProtocol::OpenAiChat,
            "m",
            "a.txt",
            "txt",
            Some(&long),
            &[],
            0.3,
        );
        let content = body["messages"][1]["content"].as_str().unwrap();
        let seg = content.split("内容片段：").nth(1).unwrap();
        let seg = seg.split('\n').next().unwrap();
        assert_eq!(seg.chars().count(), EXCERPT_MAX_CHARS, "片段应被裁剪到上限");
    }

    #[test]
    fn 解析正常响应() {
        let raw = r#"{"summary": "这是大一上学期的线性代数教材", "categories": ["数学", "学习"]}"#;
        let s = parse_suggestion(raw).unwrap();
        assert_eq!(s.summary, "这是大一上学期的线性代数教材");
        assert_eq!(s.suggested_categories, vec!["数学", "学习"]);
    }

    #[test]
    fn 解析包裹在代码块里的响应() {
        let raw = "```json\n{\"summary\": \"期末论文草稿\", \"categories\": [\"毕设\", \"论文\"]}\n```";
        let s = parse_suggestion(raw).unwrap();
        assert_eq!(s.summary, "期末论文草稿");
        assert_eq!(s.suggested_categories, vec!["毕设", "论文"]);
    }

    #[test]
    fn 兼容旧单分类响应格式() {
        let raw = r#"{"summary": "线性代数教材", "category": "数学"}"#;
        let s = parse_suggestion(raw).unwrap();
        assert_eq!(s.suggested_categories, vec!["数学"]);
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
        assert!(s.suggested_categories.is_empty());
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
    fn 三种协议组装正确地址() {
        assert_eq!(
            api_url(AiProtocol::OpenAiChat, "https://api.example.com/v1/"),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            api_url(
                AiProtocol::OpenAiChat,
                "https://api.example.com/v1/chat/completions"
            ),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            api_url(AiProtocol::OpenAiResponses, "https://api.example.com/v1"),
            "https://api.example.com/v1/responses"
        );
        assert_eq!(
            api_url(AiProtocol::AnthropicMessages, "https://api.example.com/v1/"),
            "https://api.example.com/v1/messages"
        );
    }

    #[test]
    fn 三种协议请求体结构正确() {
        let chat = build_request_body(
            AiProtocol::OpenAiChat,
            "chat-model",
            "a.txt",
            "txt",
            None,
            &[],
            0.3,
        );
        assert_eq!(chat["messages"][0]["role"], "system");
        assert_eq!(chat["messages"][1]["role"], "user");

        let responses = build_request_body(
            AiProtocol::OpenAiResponses,
            "responses-model",
            "a.txt",
            "txt",
            None,
            &[],
            0.3,
        );
        assert!(responses["instructions"].as_str().is_some());
        assert!(responses["input"].as_str().unwrap().contains("a.txt"));
        assert!(responses.get("messages").is_none());

        let anthropic = build_request_body(
            AiProtocol::AnthropicMessages,
            "claude-model",
            "a.txt",
            "txt",
            None,
            &[],
            0.3,
        );
        assert!(anthropic["system"].as_str().is_some());
        assert_eq!(anthropic["messages"][0]["role"], "user");
        assert_eq!(anthropic["max_tokens"], 512);
    }

    #[test]
    fn 三种协议响应均可解析() {
        let chat = serde_json::json!({
            "choices": [{ "message": { "content": "{\"summary\":\"聊天响应\"}" } }]
        });
        assert_eq!(
            extract_content(AiProtocol::OpenAiChat, &chat).unwrap(),
            "{\"summary\":\"聊天响应\"}"
        );

        let responses = serde_json::json!({
            "output": [{
                "content": [{
                    "type": "output_text",
                    "text": "{\"summary\":\"响应接口\"}"
                }]
            }]
        });
        assert_eq!(
            extract_content(AiProtocol::OpenAiResponses, &responses).unwrap(),
            "{\"summary\":\"响应接口\"}"
        );

        let anthropic = serde_json::json!({
            "content": [
                { "type": "thinking", "text": "ignored" },
                { "type": "text", "text": "{\"summary\":\"Anthropic 响应\"}" }
            ]
        });
        assert_eq!(
            extract_content(AiProtocol::AnthropicMessages, &anthropic).unwrap(),
            "{\"summary\":\"Anthropic 响应\"}"
        );
    }

    #[test]
    fn 解析_openai_chat_token_用量() {
        let response = serde_json::json!({
            "usage": {
                "prompt_tokens": 120,
                "completion_tokens": 30,
                "total_tokens": 150,
                "prompt_tokens_details": { "cached_tokens": 24 },
                "completion_tokens_details": { "reasoning_tokens": 12 }
            }
        });
        let usage = extract_usage(AiProtocol::OpenAiChat, &response).unwrap();
        assert_eq!(usage.input_tokens, 120);
        assert_eq!(usage.output_tokens, 30);
        assert_eq!(usage.total_tokens, 150);
        assert_eq!(usage.reasoning_tokens, Some(12));
        assert_eq!(usage.cached_tokens, Some(24));
    }

    #[test]
    fn 解析_openai_responses_token_用量() {
        let response = serde_json::json!({
            "usage": {
                "input_tokens": 100,
                "output_tokens": 20,
                "total_tokens": 120,
                "output_tokens_details": { "reasoning_tokens": 7 }
            }
        });
        let usage = extract_usage(AiProtocol::OpenAiResponses, &response).unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 20);
        assert_eq!(usage.total_tokens, 120);
        assert_eq!(usage.reasoning_tokens, Some(7));
    }

    #[test]
    fn 解析_anthropic_token_用量并回退合计() {
        let response = serde_json::json!({
            "usage": {
                "input_tokens": 80,
                "output_tokens": 16,
                "cache_creation_input_tokens": 8,
                "cache_read_input_tokens": 32
            }
        });
        let usage = extract_usage(AiProtocol::AnthropicMessages, &response).unwrap();
        assert_eq!(usage.input_tokens, 120);
        assert_eq!(usage.output_tokens, 16);
        assert_eq!(usage.total_tokens, 136);
        assert_eq!(usage.cached_tokens, Some(32));
    }

    #[test]
    fn 没有_usage_时不伪造用量() {
        let response = serde_json::json!({ "choices": [] });
        assert!(extract_usage(AiProtocol::OpenAiChat, &response).is_none());
    }
}
