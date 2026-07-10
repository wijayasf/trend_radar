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
};
use crate::services::duckdb_service;
use crate::utils::config::{self, THREADS_ACCESS_TOKEN_ENV};

const THREADS_KEYWORD_SEARCH_ENDPOINT: &str = "https://graph.threads.net/v1.0/keyword_search";
const THREADS_SEARCH_FIELDS: &str = "id,text,media_type,permalink,timestamp";
const SAMPLE_THREADS_POSTS_PATH: &str = "data/sample_threads_posts.json";
const PERMISSION_ERROR_MESSAGE: &str = "Threads keyword search permission missing. Add threads_keyword_search in Meta Developer Permissions and regenerate token.";

pub fn collect_threads_by_keyword(keyword: String) -> Result<ThreadsCollectionResult, String> {
    let normalized_keyword = keyword.trim().to_string();
    if normalized_keyword.is_empty() {
        return Err("Keyword is required.".to_string());
    }

    let access_token = read_access_token()?;
    let response_json = search_keyword(&normalized_keyword, &access_token)?;
    let posts = parse_keyword_search_response(&response_json)?;
    let fetched_count = posts.len();
    let saved_count = duckdb_service::save_threads_raw_posts(&posts)?;

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
        .query(&[
            ("q", keyword),
            ("fields", THREADS_SEARCH_FIELDS),
            ("access_token", access_token),
        ])
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

// TODO: Add pagination when the exact paging contract and storage strategy are confirmed.
