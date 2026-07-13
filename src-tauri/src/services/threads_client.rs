use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::Value;

use crate::models::threads::{
    SampleThreadsImportResult, SampleThreadsPost, ThreadPostRaw, ThreadsApiError,
    ThreadsApiErrorEnvelope, ThreadsApiPost, ThreadsCollectionResult, ThreadsKeywordSearchResponse,
    ThreadsSearchResult,
};
use crate::services::duckdb_service;
use crate::utils::config::{self, THREADS_ACCESS_TOKEN_ENV};

const THREADS_KEYWORD_SEARCH_ENDPOINT: &str = "https://graph.threads.net/v1.0/keyword_search";
const THREADS_DETAIL_ENDPOINT_BASE: &str = "https://graph.threads.net/v1.0";
const THREADS_DETAIL_FIELDS: &str = "id,text,media_type,permalink,timestamp,username,owner";
const SAMPLE_THREADS_POSTS_PATH: &str = "data/sample_threads_posts.json";
const PERMISSION_ERROR_MESSAGE: &str = "Threads keyword search permission missing. Add threads_keyword_search in Meta Developer Permissions and regenerate token.";

pub fn collect_threads_by_keyword(keyword: String) -> Result<ThreadsCollectionResult, String> {
    let normalized_keyword = keyword.trim().to_string();
    if normalized_keyword.is_empty() {
        return Err("Keyword is required.".to_string());
    }

    let search_result = search_threads_posts_by_keyword(&normalized_keyword)?;
    let fetched_count = search_result.posts.len();
    let saved_count = duckdb_service::save_threads_raw_posts(&search_result.posts)?;

    let message = if fetched_count == 0 {
        "No Threads posts returned for keyword.".to_string()
    } else {
        format!("Saved {saved_count} of {fetched_count} fetched Threads posts.")
    };

    Ok(ThreadsCollectionResult {
        keyword: normalized_keyword,
        fetched_count,
        saved_count,
        message,
    })
}

pub fn search_threads_posts_by_keyword(keyword: &str) -> Result<ThreadsSearchResult, String> {
    let normalized_keyword = keyword.trim();
    if normalized_keyword.is_empty() {
        return Err("Keyword is required.".to_string());
    }

    #[cfg(test)]
    if mock_id_only_detail_enabled() {
        return search_threads_posts_by_keyword_mock_id_only(normalized_keyword);
    }

    let access_token = read_access_token()?;
    let response_json = search_keyword(normalized_keyword, &access_token)?;
    let posts = parse_keyword_search_response(&response_json)?;
    resolve_id_only_posts(posts, &access_token, "real_threads")
}

pub fn fetch_thread_post_detail(post_id: &str) -> Result<ThreadPostRaw, String> {
    let normalized_post_id = post_id.trim();
    if normalized_post_id.is_empty() {
        return Err("Threads post ID is required.".to_string());
    }

    let access_token = read_access_token()?;
    fetch_thread_post_detail_with_token(normalized_post_id, &access_token)
}

pub fn import_sample_threads_posts() -> Result<SampleThreadsImportResult, String> {
    let sample_path = find_sample_threads_posts_path().ok_or_else(|| {
        let attempted_paths = sample_threads_posts_path_candidates()
            .into_iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");

        format!("Sample Threads posts file not found. Checked: {attempted_paths}")
    })?;
    let sample_text = fs::read_to_string(&sample_path).map_err(|error| {
        format!(
            "Failed to read sample Threads posts at {}: {error}",
            sample_path.display()
        )
    })?;
    let sample_posts = serde_json::from_str::<Vec<SampleThreadsPost>>(&sample_text)
        .map_err(|error| format!("Sample Threads posts JSON is invalid: {error}"))?;
    let loaded_count = sample_posts.len();
    let posts = sample_posts
        .into_iter()
        .map(SampleThreadsPost::into_raw_post)
        .collect::<Vec<_>>();
    let saved_count = duckdb_service::save_threads_raw_posts(&posts)?;

    Ok(SampleThreadsImportResult {
        loaded_count,
        saved_count,
        message: format!("Imported {saved_count} of {loaded_count} sample Threads posts."),
    })
}

pub fn load_sample_threads_raw_posts() -> Result<Vec<ThreadPostRaw>, String> {
    let sample_path = find_sample_threads_posts_path().ok_or_else(|| {
        let attempted_paths = sample_threads_posts_path_candidates()
            .into_iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");

        format!("Sample Threads posts file not found. Checked: {attempted_paths}")
    })?;
    let sample_text = fs::read_to_string(&sample_path).map_err(|error| {
        format!(
            "Failed to read sample Threads posts at {}: {error}",
            sample_path.display()
        )
    })?;
    let sample_posts = serde_json::from_str::<Vec<SampleThreadsPost>>(&sample_text)
        .map_err(|error| format!("Sample Threads posts JSON is invalid: {error}"))?;

    Ok(sample_posts
        .into_iter()
        .map(SampleThreadsPost::into_raw_post)
        .collect())
}

fn find_sample_threads_posts_path() -> Option<PathBuf> {
    sample_threads_posts_path_candidates()
        .into_iter()
        .find(|path| path.exists())
}

fn sample_threads_posts_path_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    candidates.push(config::project_root().join(SAMPLE_THREADS_POSTS_PATH));

    if let Ok(current_dir) = env::current_dir() {
        candidates.push(current_dir.join(SAMPLE_THREADS_POSTS_PATH));
        candidates.push(current_dir.join("..").join(SAMPLE_THREADS_POSTS_PATH));
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest_dir.join("..").join(SAMPLE_THREADS_POSTS_PATH));

    candidates
}

fn read_access_token() -> Result<String, String> {
    config::load_env_files_once();

    env::var(THREADS_ACCESS_TOKEN_ENV)
        .ok()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
        .ok_or_else(|| {
            format!(
                "Threads access token is not configured. Add {THREADS_ACCESS_TOKEN_ENV} to your local .env or environment."
            )
        })
}

fn search_keyword(keyword: &str, access_token: &str) -> Result<Value, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("Threads HTTP client initialization failed: {error}"))?;

    let response = client
        .get(THREADS_KEYWORD_SEARCH_ENDPOINT)
        .bearer_auth(access_token)
        .query(&[("q", keyword), ("media_type", "TEXT")])
        .send()
        .map_err(|_| {
            "Threads keyword search request failed before receiving a response.".to_string()
        })?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|error| format!("Threads response body read failed: {error}"))?;

    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err("Threads API rate limit reached. Wait before retrying.".to_string());
    }

    let body_json = serde_json::from_str::<Value>(&body)
        .map_err(|error| format!("Threads API returned non-JSON response: {error}"))?;

    if !status.is_success() {
        return Err(format_api_error(status, &body_json));
    }

    if let Ok(error_envelope) = serde_json::from_value::<ThreadsApiErrorEnvelope>(body_json.clone())
    {
        return Err(format_threads_error(
            "Threads API returned an error",
            &error_envelope,
        ));
    }

    Ok(body_json)
}

fn fetch_thread_post_detail_with_token(
    post_id: &str,
    access_token: &str,
) -> Result<ThreadPostRaw, String> {
    #[cfg(test)]
    if mock_id_only_detail_enabled() {
        let detail_json = mock_thread_post_detail_response(post_id)?;
        let api_post =
            serde_json::from_value::<ThreadsApiPost>(detail_json.clone()).map_err(|error| {
                format!("Threads post detail response shape is unexpected: {error}")
            })?;
        return Ok(api_post.into_raw_post(detail_json.to_string()));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("Threads HTTP client initialization failed: {error}"))?;
    let endpoint = format!("{THREADS_DETAIL_ENDPOINT_BASE}/{post_id}");

    let response = client
        .get(endpoint)
        .bearer_auth(access_token)
        .query(&[("fields", THREADS_DETAIL_FIELDS)])
        .send()
        .map_err(|_| {
            "Threads post detail request failed before receiving a response.".to_string()
        })?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|error| format!("Threads post detail response body read failed: {error}"))?;

    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err("Threads API rate limit reached while fetching post detail.".to_string());
    }

    let body_json = serde_json::from_str::<Value>(&body)
        .map_err(|error| format!("Threads post detail returned non-JSON response: {error}"))?;

    if !status.is_success() {
        return Err(format_api_error(status, &body_json));
    }

    if let Ok(error_envelope) = serde_json::from_value::<ThreadsApiErrorEnvelope>(body_json.clone())
    {
        return Err(format_threads_error(
            "Threads post detail returned an error",
            &error_envelope,
        ));
    }

    let api_post = serde_json::from_value::<ThreadsApiPost>(body_json.clone())
        .map_err(|error| format!("Threads post detail response shape is unexpected: {error}"))?;

    Ok(api_post.into_raw_post(body_json.to_string()))
}

fn resolve_id_only_posts(
    posts: Vec<ThreadPostRaw>,
    access_token: &str,
    mode: &str,
) -> Result<ThreadsSearchResult, String> {
    let mut result = ThreadsSearchResult {
        posts: Vec::with_capacity(posts.len()),
        mode: mode.to_string(),
        id_only_results_count: 0,
        detail_fetched_total: 0,
        detail_failed_total: 0,
        errors: Vec::new(),
    };

    for post in posts {
        if !post.text_missing {
            result.posts.push(post);
            continue;
        }

        result.id_only_results_count += 1;
        match fetch_thread_post_detail_with_token(&post.post_id, access_token) {
            Ok(detail_post) => {
                result.detail_fetched_total += 1;
                if detail_post.text_missing {
                    push_search_error(
                        &mut result.errors,
                        "Post detail fetched but text is unavailable for this post.",
                    );
                }
                result.posts.push(detail_post);
            }
            Err(error) => {
                result.detail_failed_total += 1;
                push_search_error(
                    &mut result.errors,
                    &format!("Post detail fetch failed for {}: {error}", post.post_id),
                );
                result.posts.push(post);
            }
        }
    }

    Ok(result)
}

fn parse_keyword_search_response(response_json: &Value) -> Result<Vec<ThreadPostRaw>, String> {
    let envelope = serde_json::from_value::<ThreadsKeywordSearchResponse>(response_json.clone())
        .map_err(|error| format!("Threads keyword search response shape is unexpected: {error}"))?;

    if envelope.data.is_empty() {
        return Ok(Vec::new());
    }

    let data = response_json
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| "Threads keyword search response is missing data array.".to_string())?;

    let mut posts = Vec::with_capacity(envelope.data.len());
    for item in data {
        let api_post = serde_json::from_value::<ThreadsApiPost>(item.clone())
            .map_err(|error| format!("Threads post response shape is unexpected: {error}"))?;
        posts.push(api_post.into_raw_post(item.to_string()));
    }

    Ok(posts)
}

fn push_search_error(errors: &mut Vec<String>, error: &str) {
    if errors.len() < 8 {
        errors.push(error.to_string());
    }
}

fn format_api_error(status: StatusCode, body_json: &Value) -> String {
    if let Ok(error_envelope) = serde_json::from_value::<ThreadsApiErrorEnvelope>(body_json.clone())
    {
        return format_threads_error(&format!("Threads API HTTP {status}"), &error_envelope);
    }

    format!("Threads API HTTP {status}. Response did not include a standard error payload.")
}

fn format_threads_error(prefix: &str, error_envelope: &ThreadsApiErrorEnvelope) -> String {
    let error = &error_envelope.error;
    if is_permission_error(error) {
        return format_error_diagnostic(PERMISSION_ERROR_MESSAGE, error);
    }

    let friendly_message = if error.message.to_lowercase().contains("rate") {
        format!(
            "{prefix}: {}. Treat this as a rate-limit or quota condition before retrying.",
            error.message
        )
    } else {
        format!("{prefix}: {}", error.message)
    };

    format_error_diagnostic(&friendly_message, error)
}

fn format_error_diagnostic(friendly_message: &str, error: &ThreadsApiError) -> String {
    let mut parts = vec![friendly_message.to_string()];

    if let Some(code) = error.code {
        parts.push(format!("code={code}"));
    }

    if let Some(subcode) = error.error_subcode {
        parts.push(format!("subcode={subcode}"));
    }

    if let Some(error_type) = &error.error_type {
        parts.push(format!("type={error_type}"));
    }

    parts.push(format!("message={}", error.message));
    parts.join(" ")
}

fn is_permission_error(error: &ThreadsApiError) -> bool {
    error.code == Some(10)
        || error
            .message
            .to_lowercase()
            .contains("application does not have permission")
}

#[cfg(test)]
fn mock_id_only_detail_enabled() -> bool {
    env::var("THREADS_MOCK_ID_ONLY_DETAIL")
        .ok()
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
}

#[cfg(test)]
fn search_threads_posts_by_keyword_mock_id_only(
    keyword: &str,
) -> Result<ThreadsSearchResult, String> {
    let keyword_json = serde_json::json!({
        "data": [
            { "id": "mock-detail-ponytail" },
            { "id": "mock-detail-cavemen" },
            { "id": "mock-detail-astryx" }
        ],
        "source": "mock_keyword_search_id_only",
        "keyword": keyword
    });
    let posts = parse_keyword_search_response(&keyword_json)?;
    resolve_id_only_posts(posts, "mock-token-redacted", "mock_id_only_detail")
}

#[cfg(test)]
fn mock_thread_post_detail_response(post_id: &str) -> Result<Value, String> {
    match post_id {
        "mock-detail-ponytail" => Ok(serde_json::json!({
            "id": "mock-detail-ponytail",
            "text": "Ponytail helps Claude Code workflow and feels useful for AI agent discovery.",
            "media_type": "TEXT",
            "permalink": "mock://threads/mock-detail-ponytail",
            "timestamp": "2026-07-01T09:00:00Z",
            "username": "mock_builder"
        })),
        "mock-detail-cavemen" => Ok(serde_json::json!({
            "id": "mock-detail-cavemen",
            "text": "Cavemen mode membantu agentic coding buat developer Indonesia.",
            "media_type": "TEXT",
            "permalink": "mock://threads/mock-detail-cavemen",
            "timestamp": "2026-07-02T10:00:00Z",
            "username": "mock_dev_indo"
        })),
        "mock-detail-astryx" => Ok(serde_json::json!({
            "id": "mock-detail-astryx",
            "text": "Astryx looks interesting for AI agent workflow, but still need to compare it with Cline.",
            "media_type": "TEXT",
            "permalink": "mock://threads/mock-detail-astryx",
            "timestamp": "2026-07-03T11:00:00Z",
            "owner": {
                "id": "mock-owner-astryx",
                "username": "mock_researcher"
            }
        })),
        other => Err(format!("Mock post detail not found for {other}")),
    }
}

// TODO: Add pagination when the exact paging contract and storage strategy are confirmed.
