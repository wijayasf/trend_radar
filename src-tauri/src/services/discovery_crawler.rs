use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::models::threads::{DiscoveryCrawlResult, DiscoveryKeywordsConfig, ThreadPostRaw};
use crate::services::{duckdb_service, threads_client};
use crate::utils::config;

const DISCOVERY_KEYWORDS_CONFIG_PATH: &str = "config/discovery_keywords.yml";
const DEFAULT_SEED_GROUP: &str = "all";
const DEFAULT_MAX_PER_SEED: usize = 10;
const ERROR_PREVIEW_LIMIT: usize = 8;

pub fn run_discovery_crawl(
    region_seed_group: Option<String>,
    max_per_seed: Option<usize>,
    dry_run: Option<bool>,
) -> Result<DiscoveryCrawlResult, String> {
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
    let mut duplicates_skipped = 0;
    let mut failed_seeds = 0;
    let mut errors = Vec::new();
    let mut collected_posts = Vec::new();
    let mut mode = "real_threads".to_string();

    for seed in &seeds {
        match threads_client::search_threads_posts_by_keyword(seed) {
            Ok(search_result) => {
                mode = merge_mode(&mode, &search_result.mode);
                detail_fetched_total += search_result.detail_fetched_total;
                detail_failed_total += search_result.detail_failed_total;
                id_only_results_count += search_result.id_only_results_count;
                for error in search_result.errors {
                    push_error(&mut errors, &error);
                }

                let limited_posts = search_result.posts.into_iter().take(max_per_seed);
                for post in limited_posts {
                    fetched_total += 1;
                    if post.post_id.trim().is_empty() {
                        continue;
                    }
                    if !seen_post_ids.insert(post.post_id.clone()) {
                        duplicates_skipped += 1;
                        continue;
                    }
                    if post.text.trim().is_empty() {
                        push_error(
                            &mut errors,
                            "Post detail fetched but text is unavailable for this post.",
                        );
                    }
                    collected_posts.push(post);
                }
            }
            Err(error) => {
                failed_seeds += 1;
                push_error(&mut errors, &format!("{seed}: {error}"));
            }
        }
    }

    if collected_posts.is_empty() && failed_seeds == seeds.len() {
        return run_sample_discovery(seed_group, max_per_seed, dry_run, errors);
    }

    let saved_total = if dry_run {
        0
    } else {
        duckdb_service::save_threads_raw_posts(&collected_posts)?
    };

    Ok(DiscoveryCrawlResult {
        seed_group,
        mode,
        seeds_processed: seeds.len(),
        fetched_total,
        detail_fetched_total,
        detail_failed_total,
        saved_total,
        duplicates_skipped,
        failed_seeds,
        id_only_results_count,
        errors,
        message: format!(
            "Discovery crawl processed {} seeds and saved {} unique posts.",
            seeds.len(),
            saved_total
        ),
    })
}

fn run_sample_discovery(
    seed_group: String,
    max_per_seed: usize,
    dry_run: bool,
    mut errors: Vec<String>,
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

    Ok(DiscoveryCrawlResult {
        seed_group,
        mode: "sample_mock".to_string(),
        seeds_processed: 0,
        fetched_total,
        detail_fetched_total: 0,
        detail_failed_total: 0,
        saved_total,
        duplicates_skipped,
        failed_seeds: 0,
        id_only_results_count: 0,
        errors,
        message: format!(
            "Sample/mock discovery saved {} of {} available sample posts.",
            saved_total, fetched_total
        ),
    })
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
