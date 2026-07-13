use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::models::threads::{
    DiscoveryCrawlResult, DiscoveryKeywordsConfig, DiscoverySeedResult, DiscoverySeedTestResult,
    ThreadPostRaw,
};
use crate::services::{duckdb_service, threads_client};
use crate::utils::config;

const DISCOVERY_KEYWORDS_CONFIG_PATH: &str = "config/discovery_keywords.yml";
const DEFAULT_SEED_GROUP: &str = "all";
const DEFAULT_MAX_PER_SEED: usize = 10;
const DEFAULT_MAX_PAGES_PER_SEED: usize = 2;
const ERROR_PREVIEW_LIMIT: usize = 8;

pub fn run_discovery_crawl(
    region_seed_group: Option<String>,
    max_per_seed: Option<usize>,
    dry_run: Option<bool>,
) -> Result<DiscoveryCrawlResult, String> {
    let started_instant = Instant::now();
    let started_at = unix_timestamp_millis_string();
    let run_id = format!("crawl-{started_at}");
    let seed_group = normalize_seed_group(region_seed_group);
    let max_per_seed = max_per_seed.unwrap_or(DEFAULT_MAX_PER_SEED).max(1);
    let dry_run = dry_run.unwrap_or(false);
    let seeds = load_discovery_seed_keywords(&seed_group)?;

    if seeds.is_empty() {
        return Err(format!(
            "No discovery seed keywords configured for group {seed_group}."
        ));
    }

    let mut seen_post_ids = HashSet::new();
    let mut fetched_total = 0;
    let mut detail_fetched_total = 0;
    let mut detail_failed_total = 0;
    let mut id_only_results_count = 0;
    let mut text_missing_total = 0;
    let mut duplicates_skipped = 0;
    let mut zero_result_seeds = 0;
    let mut failed_seeds = 0;
    let mut permission_limited_hint = false;
    let mut last_successful_seed = String::new();
    let mut last_error_summary = String::new();
    let mut errors = Vec::new();
    let mut collected_posts = Vec::new();
    let mut seed_results = Vec::new();
    let mut mode = "real_threads".to_string();

    for seed in &seeds {
        match threads_client::search_threads_posts_by_keyword_limited(
            seed,
            max_per_seed,
            DEFAULT_MAX_PAGES_PER_SEED,
        ) {
            Ok(search_result) => {
                mode = merge_mode(&mode, &search_result.mode);
                detail_fetched_total += search_result.detail_fetched_total;
                detail_failed_total += search_result.detail_failed_total;
                id_only_results_count += search_result.id_only_results_count;
                if search_result.posts.is_empty() {
                    zero_result_seeds += 1;
                } else {
                    last_successful_seed = seed.clone();
                }
                for error in search_result.errors {
                    push_error(&mut errors, &error);
                }

                let mut seed_saved_count = 0;
                let mut seed_duplicate_count = 0;
                let mut seed_text_missing_count = 0;
                for post in search_result.posts {
                    fetched_total += 1;
                    if post.post_id.trim().is_empty() {
                        continue;
                    }
                    if !seen_post_ids.insert(post.post_id.clone()) {
                        duplicates_skipped += 1;
                        seed_duplicate_count += 1;
                        continue;
                    }
                    if post.text.trim().is_empty() {
                        text_missing_total += 1;
                        seed_text_missing_count += 1;
                        push_error(
                            &mut errors,
                            "Post detail fetched but text is unavailable for this post.",
                        );
                    }
                    seed_saved_count += 1;
                    collected_posts.push(post);
                }

                seed_results.push(DiscoverySeedResult {
                    seed_keyword: seed.clone(),
                    region_group: seed_group.clone(),
                    search_status: if seed_saved_count == 0 && seed_duplicate_count == 0 {
                        "zero_result".to_string()
                    } else {
                        "success".to_string()
                    },
                    fetched_count: seed_saved_count + seed_duplicate_count,
                    saved_count: if dry_run { 0 } else { seed_saved_count },
                    duplicate_count: seed_duplicate_count,
                    detail_failed_count: search_result.detail_failed_total,
                    text_missing_count: seed_text_missing_count,
                    pages_fetched: search_result.pages_fetched,
                    pagination_stopped_reason: search_result.pagination_stopped_reason,
                    error_code: "none".to_string(),
                    error_message_safe: if seed_saved_count == 0 && seed_duplicate_count == 0 {
                        "Keyword search succeeded but no posts were returned for this seed."
                            .to_string()
                    } else {
                        String::new()
                    },
                });
            }
            Err(error) => {
                failed_seeds += 1;
                last_error_summary = safe_error_summary(&error);
                permission_limited_hint |= is_permission_error_summary(&error);
                push_error(&mut errors, &format!("{seed}: {error}"));
                seed_results.push(DiscoverySeedResult {
                    seed_keyword: seed.clone(),
                    region_group: seed_group.clone(),
                    search_status: if is_permission_error_summary(&error) {
                        "permission_error".to_string()
                    } else {
                        "api_error".to_string()
                    },
                    fetched_count: 0,
                    saved_count: 0,
                    duplicate_count: 0,
                    detail_failed_count: 0,
                    text_missing_count: 0,
                    pages_fetched: 0,
                    pagination_stopped_reason: "error".to_string(),
                    error_code: extract_error_code(&error),
                    error_message_safe: safe_error_summary(&error),
                });
            }
        }
    }

    if collected_posts.is_empty() && failed_seeds == seeds.len() {
        return run_sample_discovery(
            run_id,
            started_at,
            started_instant,
            seed_group,
            max_per_seed,
            dry_run,
            errors,
            seed_results,
            permission_limited_hint,
            last_error_summary,
        );
    }

    let saved_total = if dry_run {
        0
    } else {
        duckdb_service::save_threads_raw_posts(&collected_posts)?
    };
    let message = if fetched_total == 0 {
        format!(
            "success_with_zero_results: Discovery crawl processed {} seeds and found no matching posts.",
            seeds.len()
        )
    } else {
        format!(
            "Discovery crawl processed {} seeds and saved {} unique posts.",
            seeds.len(),
            saved_total
        )
    };

    let finished_at = unix_timestamp_millis_string();
    let result = DiscoveryCrawlResult {
        run_id,
        seed_group,
        max_per_seed,
        mode,
        started_at,
        finished_at,
        duration_ms: started_instant.elapsed().as_millis(),
        seeds_processed: seeds.len(),
        fetched_total,
        detail_fetched_total,
        detail_failed_total,
        text_missing_total,
        saved_total,
        duplicates_skipped,
        zero_result_seeds,
        failed_seeds,
        id_only_results_count,
        permission_limited_hint: permission_limited_hint
            || (zero_result_seeds > 0 && saved_total == 0),
        last_successful_seed,
        last_error_summary,
        seed_results,
        errors,
        message,
    };
    let _ = duckdb_service::save_crawl_run(&result);
    Ok(result)
}

fn run_sample_discovery(
    run_id: String,
    started_at: String,
    started_instant: Instant,
    seed_group: String,
    max_per_seed: usize,
    dry_run: bool,
    mut errors: Vec<String>,
    mut seed_results: Vec<DiscoverySeedResult>,
    permission_limited_hint: bool,
    last_error_summary: String,
) -> Result<DiscoveryCrawlResult, String> {
    let mut sample_posts = threads_client::load_sample_threads_raw_posts()?;
    let fetched_total = sample_posts.len();
    sample_posts.truncate(max_per_seed.max(DEFAULT_MAX_PER_SEED));
    let (deduped_posts, duplicates_skipped) = deduplicate_posts(sample_posts);
    let saved_total = if dry_run {
        0
    } else {
        duckdb_service::save_threads_raw_posts(&deduped_posts)?
    };

    push_error(
        &mut errors,
        "Real Threads discovery unavailable; imported sample/mock discovery posts instead.",
    );

    seed_results.push(DiscoverySeedResult {
        seed_keyword: "sample_threads_posts".to_string(),
        region_group: seed_group.clone(),
        search_status: "success".to_string(),
        fetched_count: fetched_total,
        saved_count: saved_total,
        duplicate_count: duplicates_skipped,
        detail_failed_count: 0,
        text_missing_count: 0,
        pages_fetched: 0,
        pagination_stopped_reason: "sample_mock".to_string(),
        error_code: "none".to_string(),
        error_message_safe: "Real Threads discovery unavailable; sample/mock posts were used."
            .to_string(),
    });

    let finished_at = unix_timestamp_millis_string();
    let result = DiscoveryCrawlResult {
        run_id,
        seed_group,
        max_per_seed,
        mode: "sample_mock".to_string(),
        started_at,
        finished_at,
        duration_ms: started_instant.elapsed().as_millis(),
        seeds_processed: 0,
        fetched_total,
        detail_fetched_total: 0,
        detail_failed_total: 0,
        text_missing_total: 0,
        saved_total,
        duplicates_skipped,
        zero_result_seeds: 0,
        failed_seeds: 0,
        id_only_results_count: 0,
        permission_limited_hint,
        last_successful_seed: "sample_threads_posts".to_string(),
        last_error_summary,
        seed_results,
        errors,
        message: format!(
            "Sample/mock discovery saved {} of {} available sample posts.",
            saved_total, fetched_total
        ),
    };
    let _ = duckdb_service::save_crawl_run(&result);
    Ok(result)
}

pub fn test_discovery_seed(keyword: String) -> Result<DiscoverySeedTestResult, String> {
    let seed_keyword = keyword.trim().to_string();
    if seed_keyword.is_empty() {
        return Err("Seed keyword is required.".to_string());
    }

    match threads_client::search_threads_posts_by_keyword_limited(
        &seed_keyword,
        DEFAULT_MAX_PER_SEED,
        1,
    ) {
        Ok(search_result) => {
            let fetched_count = search_result.posts.len();
            let text_available_count = search_result
                .posts
                .iter()
                .filter(|post| !post.text.trim().is_empty())
                .count();
            let sample_text_snippet = search_result
                .posts
                .iter()
                .find(|post| !post.text.trim().is_empty())
                .map(|post| safe_snippet(&post.text))
                .unwrap_or_default();

            Ok(DiscoverySeedTestResult {
                seed_keyword,
                status: if fetched_count == 0 {
                    "zero_result".to_string()
                } else {
                    "success".to_string()
                },
                fetched_count,
                detail_fetched_count: search_result.detail_fetched_total,
                text_available_count,
                sample_text_snippet,
                error_summary: search_result.errors.join(" | "),
            })
        }
        Err(error) => Ok(DiscoverySeedTestResult {
            seed_keyword,
            status: if is_permission_error_summary(&error) {
                "permission_error".to_string()
            } else {
                "api_error".to_string()
            },
            fetched_count: 0,
            detail_fetched_count: 0,
            text_available_count: 0,
            sample_text_snippet: String::new(),
            error_summary: safe_error_summary(&error),
        }),
    }
}

fn deduplicate_posts(posts: Vec<ThreadPostRaw>) -> (Vec<ThreadPostRaw>, usize) {
    let mut seen = HashSet::new();
    let mut duplicates = 0;
    let mut deduped = Vec::new();

    for post in posts {
        if post.post_id.trim().is_empty() {
            continue;
        }
        if seen.insert(post.post_id.clone()) {
            deduped.push(post);
        } else {
            duplicates += 1;
        }
    }

    (deduped, duplicates)
}

fn load_discovery_seed_keywords(seed_group: &str) -> Result<Vec<String>, String> {
    let config_path = find_discovery_keywords_config_path().ok_or_else(|| {
        format!("Could not find {DISCOVERY_KEYWORDS_CONFIG_PATH} from the app working directory")
    })?;
    let config_text = fs::read_to_string(&config_path).map_err(|error| {
        format!(
            "Failed to read discovery keywords config at {}: {error}",
            config_path.display()
        )
    })?;
    let config =
        serde_yaml::from_str::<DiscoveryKeywordsConfig>(&config_text).map_err(|error| {
            format!(
                "Failed to parse discovery keywords config at {}: {error}",
                config_path.display()
            )
        })?;

    config.ai_agent_discovery.seeds_for_group(seed_group)
}

fn find_discovery_keywords_config_path() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        config::project_root().join(DISCOVERY_KEYWORDS_CONFIG_PATH),
        PathBuf::from(DISCOVERY_KEYWORDS_CONFIG_PATH),
        PathBuf::from("..").join(DISCOVERY_KEYWORDS_CONFIG_PATH),
        manifest_dir.join("..").join(DISCOVERY_KEYWORDS_CONFIG_PATH),
    ];

    candidates.into_iter().find(|candidate| candidate.exists())
}

fn normalize_seed_group(region_seed_group: Option<String>) -> String {
    region_seed_group
        .map(|group| group.trim().to_lowercase())
        .filter(|group| !group.is_empty())
        .unwrap_or_else(|| DEFAULT_SEED_GROUP.to_string())
}

fn merge_mode(current: &str, next: &str) -> String {
    if current == next {
        current.to_string()
    } else if current == "real_threads" {
        next.to_string()
    } else {
        "mixed".to_string()
    }
}

fn push_error(errors: &mut Vec<String>, error: &str) {
    if errors.len() < ERROR_PREVIEW_LIMIT {
        errors.push(error.to_string());
    }
}

fn unix_timestamp_millis_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn extract_error_code(error: &str) -> String {
    error
        .split_whitespace()
        .find_map(|part| part.strip_prefix("code="))
        .map(ToString::to_string)
        .unwrap_or_else(|| "none".to_string())
}

fn is_permission_error_summary(error: &str) -> bool {
    error.contains("code=10")
        || error
            .to_lowercase()
            .contains("keyword search permission missing")
        || error
            .to_lowercase()
            .contains("application does not have permission")
}

fn safe_error_summary(error: &str) -> String {
    error.replace('\n', " ").chars().take(260).collect()
}

fn safe_snippet(text: &str) -> String {
    text.replace('\n', " ").chars().take(220).collect()
}
