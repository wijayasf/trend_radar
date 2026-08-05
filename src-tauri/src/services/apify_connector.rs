use std::collections::HashSet;
use std::env;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::Value;

use crate::models::threads::{
    ApifyDiscoveryResult, ApifyFilterReasons, ApifyFilteredPostSample, ApifyIncludedPostSample,
    ThreadPostRaw,
};
use crate::services::{duckdb_service, entity_detector};
use crate::utils::config;

const APIFY_TOKEN_ENV: &str = "APIFY_TOKEN";
const APIFY_THREADS_ACTOR_ID_ENV: &str = "APIFY_THREADS_ACTOR_ID";
const APIFY_RUN_TIMEOUT_SECONDS_ENV: &str = "APIFY_RUN_TIMEOUT_SECONDS";
const DEFAULT_APIFY_THREADS_ACTOR_ID: &str = "futurizerush/meta-threads-scraper";
const APIFY_SOURCE_TYPE: &str = "apify_threads_scraper";
const DEFAULT_MAX_PER_SEED: usize = 10;
const MIN_APIFY_MAX_POSTS: usize = 10;
const DEFAULT_APIFY_RUN_TIMEOUT_SECONDS: u64 = 300;
const MIN_APIFY_RUN_TIMEOUT_SECONDS: u64 = 30;
const MAX_APIFY_RUN_TIMEOUT_SECONDS: u64 = 900;
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

const AMBIGUOUS_TERMS: &[&str] = &["ponytail", "caveman", "cavemen"];
const GENERIC_AI_AGENT_TERMS: &[&str] = &["ai agent", "ai agents", "agentic ai"];
const GENERIC_MCP_TERMS: &[&str] = &["mcp", "mcp server", "model context protocol"];
const THREADBAIT_TERMS: &[&str] = &[
    "save for later",
    "entire business",
    "team of 5 ai agents",
    "here's what they do",
    "heres what they do",
];
const RECRUITMENT_TERMS: &[&str] = &[
    "looking for",
    "hiring",
    "appointment setter",
    "setter closer",
    "closer",
    "commission",
    "commission based",
    "full time",
    "ote",
    "salary",
    "job",
    "role",
    "position",
    "apply",
    "candidate",
    "recruiting",
];

pub fn run_apify_discovery_crawl(
    seeds: Option<Vec<String>>,
    max_per_seed: Option<usize>,
) -> Result<ApifyDiscoveryResult, String> {
    let seeds = normalize_seeds(seeds);
    if seeds.is_empty() {
        return Err("At least one Apify seed keyword is required.".to_string());
    }

    let actor_id = read_actor_id();
    let max_posts = normalize_max_posts(max_per_seed);
    let entity_gate = entity_detector::NamedEntityGateDetector::load()?;
    let (items, actor_run_id) = call_apify_actor(&actor_id, &seeds, max_posts)?;
    let fetched_total = items.len();
    let normalized = normalize_filter_and_dedupe_items(items, &entity_gate);
    let saved_total = duckdb_service::save_threads_raw_posts(&normalized.posts)?;

    Ok(ApifyDiscoveryResult {
        mode: APIFY_SOURCE_TYPE.to_string(),
        actor_id,
        actor_run_id,
        fetched_total,
        filtered_out_total: normalized.filtered_out_total(),
        saved_total,
        duplicates_skipped: normalized.filter_reasons.duplicate,
        entity_gate_included_total: normalized.entity_gate_included_total,
        entity_gate_filtered_total: normalized.entity_gate_filtered_total,
        filtered_out_by_reason: normalized.filter_reasons,
        sample_filtered_out: normalized.sample_filtered_out,
        sample_included: normalized.sample_included,
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
    let timeout_seconds = read_run_timeout_seconds();
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .build()
        .map_err(|error| format!("Apify HTTP client initialization failed: {error}"))?;

    let response = client
        .post(endpoint)
        .bearer_auth(token)
        .json(&input)
        .send()
        .map_err(|error| {
            if error.is_timeout() {
                format!(
                    "Apify actor is still running or timed out after {timeout_seconds} seconds. Try again with fewer seeds or wait longer."
                )
            } else {
                "Apify actor request failed before receiving a response.".to_string()
            }
        })?;

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

fn normalize_filter_and_dedupe_items(
    items: Vec<Value>,
    entity_gate: &entity_detector::NamedEntityGateDetector,
) -> NormalizedApifyItems {
    let mut filter_reasons = ApifyFilterReasons::default();
    let mut seen_ids = HashSet::new();
    let mut posts = Vec::new();
    let mut sample_filtered_out = Vec::new();
    let mut sample_included = Vec::new();
    let mut entity_gate_included_total = 0;
    let mut entity_gate_filtered_total = 0;

    for item in items {
        let text = string_field(&item, "text_content");
        let detected_entities = match entity_gate_decision(&text, entity_gate) {
            EntityGateDecision::Include(detected_entities) => {
                entity_gate_included_total += 1;
                detected_entities
            }
            EntityGateDecision::Exclude(reason) => {
                entity_gate_filtered_total += 1;
                record_filter_reason(&mut filter_reasons, reason);
                push_filtered_sample(&mut sample_filtered_out, &text, reason);
                continue;
            }
        };

        let external_id = post_external_id(&item);
        if external_id.is_empty() || !seen_ids.insert(external_id.clone()) {
            filter_reasons.duplicate += 1;
            push_filtered_sample(&mut sample_filtered_out, &text, FilterReason::Duplicate);
            continue;
        }

        let source_seed_keyword = string_field(&item, "search_keyword");
        let permalink = string_field(&item, "post_url");
        if sample_included.len() < SAMPLE_LIMIT {
            sample_included.push(ApifyIncludedPostSample {
                post_id: external_id.clone(),
                text_snippet: safe_snippet(&text),
                source_seed_keyword: source_seed_keyword.clone(),
                permalink: permalink.clone(),
                detected_entities,
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
        entity_gate_included_total,
        entity_gate_filtered_total,
        sample_filtered_out,
        sample_included,
    }
}

fn entity_gate_decision(
    text: &str,
    entity_gate: &entity_detector::NamedEntityGateDetector,
) -> EntityGateDecision {
    let normalized = text.trim().to_lowercase();
    if normalized.is_empty() {
        return EntityGateDecision::Exclude(FilterReason::EmptyText);
    }

    let detected_entities = entity_gate
        .detect(text)
        .into_iter()
        .map(|entity| entity.entity_name)
        .collect::<Vec<_>>();
    if !detected_entities.is_empty() {
        return EntityGateDecision::Include(detected_entities);
    }

    if is_recruitment_or_job_post(text) {
        return EntityGateDecision::Exclude(FilterReason::RecruitmentOrJobPost);
    }

    if THREADBAIT_TERMS
        .iter()
        .any(|term| normalized.contains(term))
    {
        return EntityGateDecision::Exclude(FilterReason::GenericThreadbait);
    }

    if GENERIC_MCP_TERMS
        .iter()
        .any(|term| contains_context_term(&normalized, term))
    {
        return EntityGateDecision::Exclude(FilterReason::GenericMcpOnly);
    }

    if GENERIC_AI_AGENT_TERMS
        .iter()
        .any(|term| contains_context_term(&normalized, term))
    {
        return EntityGateDecision::Exclude(FilterReason::GenericAiAgentOnly);
    }

    if AMBIGUOUS_TERMS
        .iter()
        .any(|term| contains_context_term(&normalized, term))
    {
        return EntityGateDecision::Exclude(FilterReason::AmbiguousWithoutEntity);
    }

    EntityGateDecision::Exclude(FilterReason::NoNamedEntity)
}

fn record_filter_reason(reasons: &mut ApifyFilterReasons, reason: FilterReason) {
    match reason {
        FilterReason::NoNamedEntity => reasons.no_named_entity += 1,
        FilterReason::RecruitmentOrJobPost => reasons.recruitment_or_job_post += 1,
        FilterReason::GenericMcpOnly => reasons.generic_mcp_only += 1,
        FilterReason::GenericAiAgentOnly => reasons.generic_ai_agent_only += 1,
        FilterReason::GenericThreadbait => reasons.generic_threadbait += 1,
        FilterReason::AmbiguousWithoutEntity => reasons.ambiguous_without_entity += 1,
        FilterReason::EmptyText => reasons.empty_text += 1,
        FilterReason::Duplicate => reasons.duplicate += 1,
    }
}

fn push_filtered_sample(
    samples: &mut Vec<ApifyFilteredPostSample>,
    text: &str,
    reason: FilterReason,
) {
    if samples.len() < SAMPLE_LIMIT {
        samples.push(ApifyFilteredPostSample {
            text_snippet: safe_snippet(text),
            reason: reason.as_str().to_string(),
        });
    }
}

fn contains_context_term(text: &str, term: &str) -> bool {
    if term == "ai" {
        return text
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|token| token == "ai");
    }

    text.contains(term)
}

fn is_recruitment_or_job_post(text: &str) -> bool {
    let normalized = normalize_gate_text(text);
    let searchable_text = format!(" {normalized} ");
    RECRUITMENT_TERMS.iter().any(|term| {
        let searchable_term = format!(" {term} ");
        searchable_text.contains(&searchable_term)
    })
}

fn normalize_gate_text(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut previous_was_space = true;

    for character in text.chars() {
        if character.is_alphanumeric() {
            for lowercase in character.to_lowercase() {
                normalized.push(lowercase);
            }
            previous_was_space = false;
        } else if !previous_was_space {
            normalized.push(' ');
            previous_was_space = true;
        }
    }

    normalized.trim().to_string()
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

fn normalize_max_posts(max_per_seed: Option<usize>) -> usize {
    max_per_seed
        .unwrap_or(DEFAULT_MAX_PER_SEED)
        .max(MIN_APIFY_MAX_POSTS)
}

fn read_run_timeout_seconds() -> u64 {
    config::load_env_files_once();

    let configured = env::var(APIFY_RUN_TIMEOUT_SECONDS_ENV).ok();
    normalize_run_timeout_seconds(configured.as_deref())
}

fn normalize_run_timeout_seconds(configured: Option<&str>) -> u64 {
    configured
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_APIFY_RUN_TIMEOUT_SECONDS)
        .clamp(MIN_APIFY_RUN_TIMEOUT_SECONDS, MAX_APIFY_RUN_TIMEOUT_SECONDS)
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
    entity_gate_included_total: usize,
    entity_gate_filtered_total: usize,
    sample_filtered_out: Vec<ApifyFilteredPostSample>,
    sample_included: Vec<ApifyIncludedPostSample>,
}

impl NormalizedApifyItems {
    fn filtered_out_total(&self) -> usize {
        self.entity_gate_filtered_total + self.filter_reasons.duplicate
    }
}

enum EntityGateDecision {
    Include(Vec<String>),
    Exclude(FilterReason),
}

#[derive(Clone, Copy)]
enum FilterReason {
    NoNamedEntity,
    RecruitmentOrJobPost,
    GenericMcpOnly,
    GenericAiAgentOnly,
    GenericThreadbait,
    AmbiguousWithoutEntity,
    EmptyText,
    Duplicate,
}

impl FilterReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::NoNamedEntity => "no_named_entity",
            Self::RecruitmentOrJobPost => "recruitment_or_job_post",
            Self::GenericMcpOnly => "generic_mcp_only",
            Self::GenericAiAgentOnly => "generic_ai_agent_only",
            Self::GenericThreadbait => "generic_threadbait",
            Self::AmbiguousWithoutEntity => "ambiguous_without_entity",
            Self::EmptyText => "empty_text",
            Self::Duplicate => "duplicate",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_apify_minimum_max_posts() {
        assert_eq!(normalize_max_posts(None), 10);
        assert_eq!(normalize_max_posts(Some(1)), 10);
        assert_eq!(normalize_max_posts(Some(9)), 10);
        assert_eq!(normalize_max_posts(Some(10)), 10);
        assert_eq!(normalize_max_posts(Some(25)), 25);
    }

    #[test]
    fn normalizes_apify_run_timeout_seconds() {
        assert_eq!(normalize_run_timeout_seconds(None), 300);
        assert_eq!(normalize_run_timeout_seconds(Some("invalid")), 300);
        assert_eq!(normalize_run_timeout_seconds(Some("10")), 30);
        assert_eq!(normalize_run_timeout_seconds(Some("450")), 450);
        assert_eq!(normalize_run_timeout_seconds(Some("1200")), 900);
    }

    #[test]
    fn requires_named_entities_for_apify_discovery() {
        let items = vec![
            serde_json::json!({
                "post_code": "generic-threadbait",
                "text_content": "I built a team of 5 AI agents that run my ENTIRE business. Here's what they do. (save for later)",
                "search_keyword": "AI Agent",
                "post_url": "https://threads.net/t/generic-threadbait"
            }),
            serde_json::json!({
                "post_code": "generic-html-mcp",
                "text_content": "HTML blocks now work with any agent via MCP.",
                "search_keyword": "MCP server",
                "post_url": "https://threads.net/t/generic-html-mcp"
            }),
            serde_json::json!({
                "post_code": "generic-mcp-server",
                "text_content": "We built an MCP server and weekly usage tripled.",
                "search_keyword": "MCP server",
                "post_url": "https://threads.net/t/generic-mcp-server"
            }),
            serde_json::json!({
                "post_code": "generic-agentic-ai",
                "text_content": "Who wants to learn Agentic AI?",
                "search_keyword": "Agentic AI",
                "post_url": "https://threads.net/t/generic-agentic-ai"
            }),
            serde_json::json!({
                "post_code": "generic-recruitment",
                "text_content": "Looking for an Appointment Setter/Closer. Full-time role with commission based OTE.",
                "search_keyword": "AI Agent",
                "post_url": "https://threads.net/t/generic-recruitment"
            }),
            serde_json::json!({
                "post_code": "named-graphify",
                "text_content": "Graphify helps reduce token usage for agent memory.",
                "search_keyword": "AI Agent",
                "post_url": "https://threads.net/t/named-graphify"
            }),
            serde_json::json!({
                "post_code": "named-headroom",
                "text_content": "Headroom is useful for managing agent workflow.",
                "search_keyword": "AI Agent",
                "post_url": "https://threads.net/t/named-headroom"
            }),
            serde_json::json!({
                "post_code": "named-ponytail-claude",
                "text_content": "Ponytail feels useful for Claude Code workflow",
                "search_keyword": "Ponytail Claude Code",
                "post_url": "https://threads.net/t/named-ponytail-claude"
            }),
            serde_json::json!({
                "post_code": "named-claude-code",
                "text_content": "Claude Code now has a built-in iOS simulator.",
                "search_keyword": "Claude Code",
                "post_url": "https://threads.net/t/named-claude-code"
            }),
            serde_json::json!({
                "post_code": "named-graphify-mcp",
                "text_content": "We built Graphify MCP server for agent memory.",
                "search_keyword": "MCP server",
                "post_url": "https://threads.net/t/named-graphify-mcp"
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
                "post_code": "ambiguous-ponytail",
                "text_content": "Who can do a ponytail braid?",
                "search_keyword": "Ponytail",
                "post_url": "https://threads.net/t/ambiguous-ponytail"
            }),
            serde_json::json!({
                "post_code": "named-graphify",
                "text_content": "Graphify helps agent memory. Duplicate result.",
                "search_keyword": "AI Agent",
                "post_url": "https://threads.net/t/named-graphify-duplicate"
            }),
        ];
        let entity_gate = entity_detector::NamedEntityGateDetector::load()
            .expect("aliases config should load for entity gate test");

        let normalized = normalize_filter_and_dedupe_items(items, &entity_gate);

        assert_eq!(normalized.posts.len(), 5);
        assert_eq!(normalized.entity_gate_included_total, 6);
        assert_eq!(normalized.entity_gate_filtered_total, 8);
        assert_eq!(normalized.filter_reasons.recruitment_or_job_post, 1);
        assert_eq!(normalized.filter_reasons.generic_threadbait, 1);
        assert_eq!(normalized.filter_reasons.generic_mcp_only, 2);
        assert_eq!(normalized.filter_reasons.generic_ai_agent_only, 1);
        assert_eq!(normalized.filter_reasons.empty_text, 1);
        assert_eq!(normalized.filter_reasons.no_named_entity, 1);
        assert_eq!(normalized.filter_reasons.ambiguous_without_entity, 1);
        assert_eq!(normalized.filter_reasons.duplicate, 1);
        assert!(normalized
            .posts
            .iter()
            .any(|post| post.post_id == "named-graphify"));
        assert!(normalized
            .posts
            .iter()
            .any(|post| post.post_id == "named-headroom"));
        assert!(normalized
            .posts
            .iter()
            .any(|post| post.post_id == "named-ponytail-claude"));
        assert!(normalized
            .posts
            .iter()
            .any(|post| post.post_id == "named-claude-code"));
        assert!(normalized
            .posts
            .iter()
            .any(|post| post.post_id == "named-graphify-mcp"));
        assert!(normalized.sample_included.iter().any(|sample| {
            sample.post_id == "named-ponytail-claude"
                && sample.detected_entities.contains(&"Ponytail".to_string())
                && sample
                    .detected_entities
                    .contains(&"Claude Code".to_string())
        }));

        assert!(matches!(
            entity_gate_decision(
                "We are hiring a developer for a Claude Code automation role.",
                &entity_gate
            ),
            EntityGateDecision::Include(_)
        ));
        assert!(matches!(
            entity_gate_decision(
                "Looking for Appointment Setter/Closer for my AI automation offer. Commission based, full-time OTE $6-$10k a month.",
                &entity_gate
            ),
            EntityGateDecision::Exclude(FilterReason::RecruitmentOrJobPost)
        ));
    }
}
