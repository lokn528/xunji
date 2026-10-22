//! AI 相关命令。
//!
//! 请求在 Rust 侧发起（见 `plan.md`）：API Key 只存在于 Rust 侧，
//! 请求体的字段裁剪也在这里完成，前端无法绕过。

use tauri::State;

use crate::ai::{self, ChatRequest};
use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::AiSuggestion;
use crate::state::AppState;

/// AI 请求超时（秒）。
const TIMEOUT_SECS: u64 = 30;

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

/// 分析一张卡片的内部实现——不写入数据库，只产出建议。
pub async fn analyze_card_inner(
    state: &AppState,
    card_id: &str,
) -> AppResult<AiSuggestion> {
    // 先同步取出所需数据，避免在 await 期间持有数据库锁。
    let (file_name, file_type, excerpt, categories, settings) = {
        let card = state
            .with_db(|conn| db::get_card(conn, card_id))?
            .ok_or_else(|| AppError::Invalid("卡片不存在".into()))?;
        let cats = state.with_db(db::list_categories)?;
        let settings = state.with_db(db::load_settings)?;
        let excerpt = if settings.ai_filename_only {
            None
        } else {
            crate::textprobe::excerpt(&card.file_path)
        };
        let names = cats.into_iter().map(|c| c.name).collect::<Vec<_>>();
        (
            crate::fsops::file_name(&card.file_path),
            crate::fsops::extension(&card.file_path),
            excerpt,
            names,
            settings,
        )
    };

    let api_key = super::settings::read_api_key()?;
    if settings.ai_base_url.trim().is_empty() || settings.ai_model.trim().is_empty() {
        return Err(AppError::AiUnavailable);
    }

    let body = ai::build_request_body(
        &settings.ai_model,
        &file_name,
        &file_type,
        excerpt.as_deref(),
        &categories,
        settings.ai_temperature,
    );

    let raw = call_chat(&settings.ai_base_url, &api_key, body).await?;
    ai::parse_suggestion(&raw)
}

/// 测试 AI 连通性。
#[tauri::command]
pub async fn test_ai_connection(state: State<'_, AppState>) -> AppResult<String> {
    let settings = state.with_db(db::load_settings)?;
    let api_key = super::settings::read_api_key()?;
    if settings.ai_base_url.trim().is_empty() || settings.ai_model.trim().is_empty() {
        return Err(AppError::AiUnavailable);
    }

    let body = ChatRequest {
        model: settings.ai_model.clone(),
        messages: vec![crate::ai::ChatMessage {
            role: "user",
            content: "ping".into(),
        }],
        temperature: 0.0,
    };

    call_chat(&settings.ai_base_url, &api_key, body).await?;
    Ok("连接正常".into())
}

/// 发起一次 chat/completions 请求，返回文本内容。
async fn call_chat(base_url: &str, api_key: &str, body: ChatRequest) -> AppResult<String> {
    let url = ai::chat_completions_url(base_url);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::AiRequestFailed(e.to_string()))?;

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
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

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|_| AppError::AiInvalidResponse)?;
    ai::extract_content(&json)
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
