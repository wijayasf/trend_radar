use std::collections::HashSet;
use std::env;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::Value;

use crate::models::threads::{
    ApifyDiscoveryResult, ApifyFilterReasons, ApifySamplePost, ThreadPostRaw,
};
use crate::services::duckdb_service;
use crate::utils::config;

const APIFY_TOKEN_ENV: &str = "APIFY_TOKEN";
const APIFY_THREADS_ACTOR_ID_ENV: &str = "APIFY_THREADS_ACTOR_ID";
const DEFAULT_APIFY_THREADS_ACTOR_ID: &str = "futurizerush/meta-threads-scraper";
const APIFY_SOURCE_TYPE: &str = "apify_threads_scraper";
const DEFAULT_MAX_PER_SEED: usize = 10;
const SAMPLE_LIMIT: usize = 6;

const DEFAULT_APIFY_SEEDS: &[&str] = &[
    "AI Agent",
    "Agentic AI",
    "Claude Code",
    "Claude Code plugin",
    "MCP server",
    "Ponytail Claude Code",
    "Cavemen Claude Code",
    "Astryx AI",
];

const AI_CONTEXT_TERMS: &[&str] = &[
    "ai",
    "agent",
    "agents",
    "agentic",
    "claude code",
    "codex",
    "mcp",
    "cursor",
    "cline",
    "coding",
    "developer",
    "workflow",
    "automation",
    "llm",
    "llms",
    "langgraph",
    "crewai",
    "autogen",
    "plugin",
    "sdk",
    "cli",
    "server",
    "framework",
    "model",
    "tool",
];

const AMBIGUOUS_TERMS: &[&str] = &["ponytail", "caveman", "cavemen"];

pub fn run_apify_discovery_crawl(
    seeds: Option<Vec<String>>,
    max_per_seed: Option<usize>,
) -> Result<ApifyDiscoveryResult, String> {
    let seeds = normalize_seeds(seeds);
    if seeds.is_empty() {
        return Err("At least one Apify seed keyword is required.".to_string());
    }

    let actor_id = read_actor_id();
    let max_posts = max_per_seed.unwrap_or(DEFAULT_MAX_PER_SEED).max(1);
    let (items, actor_run_id) = call_apify_actor(&actor_id, &seeds, max_posts)?;
    let fetched_total = items.len();
    let normalized = normalize_filter_and_dedupe_items(items);
    let saved_total = duckdb_service::save_threads_raw_posts(&normalized.posts)?;

    Ok(ApifyDiscoveryResult {
        mode: APIFY_SOURCE_TYPE.to_string(),
        actor_id,
        actor_run_id,
        fetched_total,
        filtered_out_total: normalized.filtered_out_total(),
        saved_total,
        duplicates_skipped: normalized.filter_reasons.duplicate,
        detected_relevance_count: normalized.detected_relevance_count,
        included_by_context_count: normalized.detected_relevance_count,
        filtered_out_by_reason: normalized.filter_reasons,
        sample_saved_posts: normalized.sample_saved_posts,
        safe_error_summary: String::new(),
    })
}

fn call_apify_actor(
    actor_id: &str,
    seeds: &[String],
    max_posts: usize,
) -> Result<(Vec<Value>, String), String> {
    let token = read_apify_token()?;
    let actor_url_id = actor_id.trim().replace('/', "~");
    let endpoint =
        format!("https://api.apify.com/v2/acts/{actor_url_id}/run-sync-get-dataset-items");
    let input = serde_json::json!({
        "mode": "search",
        "keywords": seeds,
        "search_filter": "recent",
        "max_posts": max_posts,
    });
    let client = Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| format!("Apify HTTP client initialization failed: {error}"))?;

    let response = client
        .post(endpoint)
        .bearer_auth(token)
        .json(&input)
        .send()
        .map_err(|_| "Apify actor request failed before receiving a response.".to_string())?;

    let status = response.status();
    let actor_run_id = response
        .headers()
        .get("x-apify-actor-run-id")
        .or_else(|| response.headers().get("apify-actor-run-id"))
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unavailable_sync_run")
        .to_string();
    let body = response
        .text()
        .map_err(|error| format!("Apify response body read failed: {error}"))?;
    let body_json = serde_json::from_str::<Value>(&body)
        .map_err(|error| format!("Apify returned non-JSON response: {error}"))?;

    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err("Apify API rate limit reached. Wait before retrying.".to_string());
    }

    if !status.is_success() {
        return Err(format_apify_error(status, &body_json));
    }

    Ok((parse_apify_items(&body_json)?, actor_run_id))
}

fn normalize_filter_and_dedupe_items(items: Vec<Value>) -> NormalizedApifyItems {
    let mut filter_reasons = ApifyFilterReasons::default();
    let mut seen_ids = HashSet::new();
    let mut posts = Vec::new();
    let mut sample_saved_posts = Vec::new();
    let mut detected_relevance_count = 0;

    for item in items {
        let text = string_field(&item, "text_content");
        match relevance_decision(&text) {
            RelevanceDecision::Include => {
                detected_relevance_count += 1;
            }
            RelevanceDecision::Exclude(reason) => {
                match reason {
                    FilterReason::EmptyText => filter_reasons.empty_text += 1,
                    FilterReason::NoAiContext => filter_reasons.no_ai_context += 1,
                    FilterReason::AmbiguousWithoutContext => {
                        filter_reasons.ambiguous_without_context += 1
                    }
                }
                continue;
            }
        }

        let external_id = post_external_id(&item);
        if external_id.is_empty() || !seen_ids.insert(external_id.clone()) {
            filter_reasons.duplicate += 1;
            continue;
        }

        let source_seed_keyword = string_field(&item, "search_keyword");
        let permalink = string_field(&item, "post_url");
        if sample_saved_posts.len() < SAMPLE_LIMIT {
            sample_saved_posts.push(ApifySamplePost {
                post_id: external_id.clone(),
                text_snippet: safe_snippet(&text),
                source_seed_keyword: source_seed_keyword.clone(),
                permalink: permalink.clone(),
            });
        }

        posts.push(ThreadPostRaw {
            post_id: external_id,
            text_missing: text.trim().is_empty(),
            text,
            author_id: None,
            author_username: non_empty_string_field(&item, "username"),
            author_display_name: non_empty_string_field(&item, "display_name"),
            media_type: Some("TEXT".to_string()),
            permalink: non_empty(permalink),
            posted_at: non_empty_string_field(&item, "created_at"),
            source_type: Some(APIFY_SOURCE_TYPE.to_string()),
            source_seed_keyword: non_empty(source_seed_keyword),
            keyword_match: value_string_field(&item, "keyword_match"),
            like_count: i64_field(&item, "like_count"),
            reply_count: i64_field(&item, "reply_count"),
            repost_count: i64_field(&item, "repost_count"),
            quote_count: i64_field(&item, "quote_count"),
            share_count: i64_field(&item, "share_count"),
            view_count: i64_field(&item, "view_count"),
            raw_json: item.to_string(),
        });
    }

    NormalizedApifyItems {
        posts,
        filter_reasons,
        detected_relevance_count,
        sample_saved_posts,
    }
}

fn relevance_decision(text: &str) -> RelevanceDecision {
    let normalized = text.trim().to_lowercase();
    if normalized.is_empty() {
        return RelevanceDecision::Exclude(FilterReason::EmptyText);
    }

    let has_ai_context = AI_CONTEXT_TERMS
        .iter()
        .any(|term| contains_context_term(&normalized, term));
    let has_ambiguous_term = AMBIGUOUS_TERMS
        .iter()
        .any(|term| contains_context_term(&normalized, term));

    if has_ambiguous_term && !has_ai_context {
        return RelevanceDecision::Exclude(FilterReason::AmbiguousWithoutContext);
    }

    if !has_ai_context {
        return RelevanceDecision::Exclude(FilterReason::NoAiContext);
    }

    RelevanceDecision::Include
}

fn contains_context_term(text: &str, term: &str) -> bool {
    if term == "ai" {
        return text
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|token| token == "ai");
    }

    text.contains(term)
}

fn parse_apify_items(body_json: &Value) -> Result<Vec<Value>, String> {
    if let Some(items) = body_json.as_array() {
        return Ok(items.clone());
    }

    for key in ["items", "data"] {
        if let Some(items) = body_json.get(key).and_then(Value::as_array) {
            return Ok(items.clone());
        }
    }

    Err("Apify actor response did not include a dataset items array.".to_string())
}

fn post_external_id(item: &Value) -> String {
    non_empty_string_field(item, "post_code")
        .or_else(|| non_empty_string_field(item, "post_url"))
        .unwrap_or_default()
}

fn string_field(item: &Value, key: &str) -> String {
    item.get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_default()
}

fn non_empty_string_field(item: &Value, key: &str) -> Option<String> {
    non_empty(string_field(item, key))
}

fn value_string_field(item: &Value, key: &str) -> Option<String> {
    item.get(key).and_then(|value| match value {
        Value::Null => None,
        Value::String(text) => non_empty(text.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        other => Some(other.to_string()),
    })
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn i64_field(item: &Value, key: &str) -> i64 {
    item.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn safe_snippet(text: &str) -> String {
    let mut snippet = text.trim().chars().take(180).collect::<String>();
    if text.trim().chars().count() > 180 {
        snippet.push_str("...");
    }
    snippet
}

fn normalize_seeds(seeds: Option<Vec<String>>) -> Vec<String> {
    let requested = seeds.unwrap_or_else(|| {
        DEFAULT_APIFY_SEEDS
            .iter()
            .map(|seed| seed.to_string())
            .collect()
    });
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for seed in requested {
        let trimmed = seed.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_lowercase()) {
            normalized.push(trimmed.to_string());
        }
    }

    normalized
}

fn read_actor_id() -> String {
    config::load_env_files_once();

    env::var(APIFY_THREADS_ACTOR_ID_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_APIFY_THREADS_ACTOR_ID.to_string())
}

fn read_apify_token() -> Result<String, String> {
    config::load_env_files_once();

    env::var(APIFY_TOKEN_ENV)
        .ok()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
        .ok_or_else(|| {
            format!(
                "Apify token is not configured. Add {APIFY_TOKEN_ENV} to your local .env or environment."
            )
        })
}

fn format_apify_error(status: StatusCode, body_json: &Value) -> String {
    let message = body_json
        .get("error")
        .and_then(|error| error.get("message").or_else(|| error.get("description")))
        .and_then(Value::as_str)
        .or_else(|| body_json.get("message").and_then(Value::as_str))
        .unwrap_or("Apify API returned an error.");

    format!("Apify API HTTP {status}: {message}")
}

struct NormalizedApifyItems {
    posts: Vec<ThreadPostRaw>,
    filter_reasons: ApifyFilterReasons,
    detected_relevance_count: usize,
    sample_saved_posts: Vec<ApifySamplePost>,
}

impl NormalizedApifyItems {
    fn filtered_out_total(&self) -> usize {
        self.filter_reasons.empty_text
            + self.filter_reasons.no_ai_context
            + self.filter_reasons.ambiguous_without_context
            + self.filter_reasons.duplicate
    }
}

enum RelevanceDecision {
    Include,
    Exclude(FilterReason),
}

enum FilterReason {
    EmptyText,
    NoAiContext,
    AmbiguousWithoutContext,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_apify_threads_results_for_ai_agent_relevance() {
        let items = vec![
            serde_json::json!({
                "post_code": "apify-ai-agent",
                "text_content": "AI Agent roadmap for developer workflow",
                "search_keyword": "AI Agent",
                "post_url": "https://threads.net/t/apify-ai-agent"
            }),
            serde_json::json!({
                "post_code": "apify-claude-code",
                "text_content": "Claude Code subscription is useful for coding automation",
                "search_keyword": "Claude Code",
                "post_url": "https://threads.net/t/apify-claude-code"
            }),
            serde_json::json!({
                "post_code": "apify-ponytail-hair",
                "text_content": "Who can do a ponytail braid?",
                "search_keyword": "Ponytail",
                "post_url": "https://threads.net/t/apify-ponytail-hair"
            }),
            serde_json::json!({
                "post_code": "apify-caveman-cartoon",
                "text_content": "Captain Caveman is on TV again",
                "search_keyword": "Cavemen",
                "post_url": "https://threads.net/t/apify-caveman-cartoon"
            }),
            serde_json::json!({
                "post_code": "apify-ponytail-claude",
                "text_content": "Ponytail feels useful for Claude Code workflow",
                "search_keyword": "Ponytail Claude Code",
                "post_url": "https://threads.net/t/apify-ponytail-claude"
            }),
            serde_json::json!({
                "post_code": "apify-cavemen-claude",
                "text_content": "Cavemen mode Claude Code keeps the coding workflow focused",
                "search_keyword": "Cavemen Claude Code",
                "post_url": "https://threads.net/t/apify-cavemen-claude"
            }),
            serde_json::json!({
                "post_code": "apify-empty",
                "text_content": "",
                "search_keyword": "AI Agent",
                "post_url": "https://threads.net/t/apify-empty"
            }),
            serde_json::json!({
                "post_code": "apify-lifestyle",
                "text_content": "Morning routine notes and coffee",
                "search_keyword": "Trend",
                "post_url": "https://threads.net/t/apify-lifestyle"
            }),
            serde_json::json!({
                "post_code": "apify-ai-agent",
                "text_content": "AI Agent roadmap duplicate",
                "search_keyword": "AI Agent",
                "post_url": "https://threads.net/t/apify-ai-agent-duplicate"
            }),
        ];

        let normalized = normalize_filter_and_dedupe_items(items);

        assert_eq!(normalized.posts.len(), 4);
        assert_eq!(normalized.detected_relevance_count, 5);
        assert_eq!(normalized.filter_reasons.empty_text, 1);
        assert_eq!(normalized.filter_reasons.no_ai_context, 1);
        assert_eq!(normalized.filter_reasons.ambiguous_without_context, 2);
        assert_eq!(normalized.filter_reasons.duplicate, 1);
        assert!(normalized
            .posts
            .iter()
            .any(|post| post.post_id == "apify-ai-agent"));
        assert!(normalized
            .posts
            .iter()
            .any(|post| post.post_id == "apify-claude-code"));
        assert!(normalized
            .posts
            .iter()
            .any(|post| post.post_id == "apify-ponytail-claude"));
        assert!(normalized
            .posts
            .iter()
            .any(|post| post.post_id == "apify-cavemen-claude"));
    }
}
