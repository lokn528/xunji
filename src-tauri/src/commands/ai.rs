//! AI 相关命令。
//!
//! 请求在 Rust 侧发起（见 `plan.md`）：API Key 只存在于 Rust 侧，
//! 请求体的字段裁剪也在这里完成，前端无法绕过。

use tauri::State;

use crate::ai;
use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{AiProtocol, AiSuggestion, AiUsage};
use crate::state::AppState;

/// AI 请求超时（秒）。
const TIMEOUT_SECS: u64 = 30;

struct AiCallResult {
    content: String,
    usage: Option<AiUsage>,
}

/// 分析一张卡片，产出说明与分类建议。
///
/// 结果只是建议——前端须先填入输入框，用户确认后才落库（constitution 原则 II）。
#[tauri::command]
pub async fn analyze_card(
    state: State<'_, AppState>,
    card_id: String,
) -> AppResult<AiSuggestion> {
    analyze_card_inner(&state, &card_id).await
}

/// 只分析一个文件路径，供导入前的「AI 智能填充」使用。
#[tauri::command]
pub async fn analyze_path(
    state: State<'_, AppState>,
    path: String,
) -> AppResult<AiSuggestion> {
    analyze_path_inner(&state, &path).await
}

/// 分析一张卡片——不写入数据库，只产出建议。
pub async fn analyze_card_inner(
    state: &AppState,
    card_id: &str,
) -> AppResult<AiSuggestion> {
    let path = state
        .with_db(|conn| db::get_card(conn, card_id))?
        .ok_or_else(|| AppError::Invalid("卡片不存在".into()))?
        .file_path;
    analyze_path_inner(state, &path).await
}

/// 按文件路径分析——卡片和导入预览共用同一套输入裁剪规则。
async fn analyze_path_inner(state: &AppState, path: &str) -> AppResult<AiSuggestion> {
    if !std::path::Path::new(path).exists() {
        return Err(AppError::SourceMissing(path.to_string()));
    }

    // 先同步取出所需数据，避免在 await 期间持有数据库锁。
    let (file_name, file_type, excerpt, categories, settings) = {
        let cats = state.with_db(db::list_categories)?;
        let settings = state.with_db(db::load_settings)?;
        let excerpt = if settings.ai_filename_only {
            None
        } else {
            crate::textprobe::excerpt(path)
        };
        let names = cats.into_iter().map(|c| c.name).collect::<Vec<_>>();
        (
            crate::fsops::file_name(path),
            crate::fsops::extension(path),
            excerpt,
            names,
            settings,
        )
    };

    if settings.ai_base_url.trim().is_empty() || settings.ai_model.trim().is_empty() {
        return Err(AppError::AiUnavailable);
    }
    let api_key = super::settings::read_api_key()?;

    let body = ai::build_request_body(
        settings.ai_protocol,
        &settings.ai_model,
        &file_name,
        &file_type,
        excerpt.as_deref(),
        &categories,
        settings.ai_temperature,
    );

    let response = call_ai(settings.ai_protocol, &settings.ai_base_url, &api_key, body).await?;
    let mut suggestion = ai::parse_suggestion(&response.content)?;
    suggestion.usage = response.usage;
    Ok(suggestion)
}

/// 测试 AI 连通性。
#[tauri::command]
pub async fn test_ai_connection(state: State<'_, AppState>) -> AppResult<String> {
    let settings = state.with_db(db::load_settings)?;
    let api_key = super::settings::read_api_key()?;
    if settings.ai_base_url.trim().is_empty() || settings.ai_model.trim().is_empty() {
        return Err(AppError::AiUnavailable);
    }

    let body = ai::build_probe_body(settings.ai_protocol, &settings.ai_model);

    let response = call_ai(settings.ai_protocol, &settings.ai_base_url, &api_key, body).await?;
    Ok(match response.usage {
        Some(usage) => format!(
            "连接正常，本次检测输入 {} / 输出 {} / 合计 {} tokens",
            usage.input_tokens, usage.output_tokens, usage.total_tokens
        ),
        None => "连接正常".into(),
    })
}

/// 按选定的协议发起请求，返回文本内容和 token 用量。
async fn call_ai(
    protocol: AiProtocol,
    base_url: &str,
    api_key: &str,
    body: serde_json::Value,
) -> AppResult<AiCallResult> {
    let url = ai::api_url(protocol, base_url);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::AiRequestFailed(e.to_string()))?;

    let request = client.post(&url).header("Content-Type", "application/json");
    let request = match protocol {
        AiProtocol::OpenAiChat | AiProtocol::OpenAiResponses => {
            request.header("Authorization", format!("Bearer {api_key}"))
        }
        AiProtocol::AnthropicMessages => request
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01"),
    };

    let resp = request
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::AiRequestFailed(describe_reqwest_error(&e)))?;

    let status = resp.status();
    if !status.is_success() {
        let detail = resp.text().await.unwrap_or_default();
        // 只回传状态码与截断后的正文，不回传可能含 key 的请求信息。
        let snippet: String = detail.chars().take(200).collect();
        return Err(AppError::AiRequestFailed(format!(
            "服务返回 {status}：{snippet}"
        )));
    }

    let json: serde_json::Value = resp.json().await.map_err(|_| AppError::AiInvalidResponse)?;
    let content = ai::extract_content(protocol, &json)?;
    let usage = ai::extract_usage(protocol, &json);
    Ok(AiCallResult { content, usage })
}

/// 把 reqwest 错误转成用户能看懂的中文。
fn describe_reqwest_error(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        "请求超时".into()
    } else if e.is_connect() {
        "无法连接到服务器，请检查 Base URL 与网络".into()
    } else {
        e.to_string()
    }
}
