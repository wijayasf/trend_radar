use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ThreadPostRaw {
    pub post_id: String,
    pub text: String,
    pub text_missing: bool,
    pub author_id: Option<String>,
    pub author_username: Option<String>,
    pub media_type: Option<String>,
    pub permalink: Option<String>,
    pub posted_at: Option<String>,
    pub raw_json: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThreadsCollectionResult {
    pub keyword: String,
    pub fetched_count: usize,
    pub saved_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SampleThreadsImportResult {
    pub loaded_count: usize,
    pub saved_count: usize,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct DiscoveryKeywordsConfig {
    pub ai_agent_discovery: DiscoveryKeywordGroups,
}

#[derive(Debug, Deserialize)]
pub struct DiscoveryKeywordGroups {
    #[serde(default)]
    pub global: Vec<String>,
    #[serde(default)]
    pub indonesia: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryCrawlResult {
    pub seed_group: String,
    pub mode: String,
    pub seeds_processed: usize,
    pub fetched_total: usize,
    pub detail_fetched_total: usize,
    pub detail_failed_total: usize,
    pub saved_total: usize,
    pub duplicates_skipped: usize,
    pub failed_seeds: usize,
    pub id_only_results_count: usize,
    pub errors: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ThreadsSearchResult {
    pub posts: Vec<ThreadPostRaw>,
    pub mode: String,
    pub id_only_results_count: usize,
    pub detail_fetched_total: usize,
    pub detail_failed_total: usize,
    pub errors: Vec<String>,
}

impl DiscoveryKeywordGroups {
    pub fn seeds_for_group(&self, group: &str) -> Result<Vec<String>, String> {
        let mut seeds = Vec::new();
        match group {
            "all" => {
                seeds.extend(self.global.clone());
                seeds.extend(self.indonesia.clone());
            }
            "global" => seeds.extend(self.global.clone()),
            "indonesia" => seeds.extend(self.indonesia.clone()),
            other => {
                return Err(format!(
                    "Unsupported discovery seed group: {other}. Use all, global, or indonesia."
                ));
            }
        }

        let mut deduped = BTreeMap::new();
        for seed in seeds {
            let trimmed = seed.trim();
            if !trimmed.is_empty() {
                deduped.entry(trimmed.to_lowercase()).or_insert(seed);
            }
        }

        Ok(deduped.into_values().collect())
    }
}

#[derive(Debug, Deserialize)]
pub struct ThreadsKeywordSearchResponse {
    #[serde(default)]
    pub data: Vec<ThreadsApiPost>,
}

#[derive(Debug, Deserialize)]
pub struct ThreadsApiPost {
    pub id: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub media_type: Option<String>,
    #[serde(default)]
    pub permalink: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub owner: Option<ThreadsApiOwner>,
}

#[derive(Debug, Deserialize)]
pub struct ThreadsApiOwner {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SampleThreadsPost {
    pub post_id: String,
    pub text: String,
    #[serde(default)]
    pub media_type: Option<String>,
    #[serde(default)]
    pub permalink: Option<String>,
    #[serde(default)]
    pub posted_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ThreadsApiErrorEnvelope {
    pub error: ThreadsApiError,
}

#[derive(Debug, Deserialize)]
pub struct ThreadsApiError {
    pub message: String,
    #[serde(default)]
    pub code: Option<i64>,
    #[serde(default)]
    pub error_subcode: Option<i64>,
    #[serde(rename = "type", default)]
    pub error_type: Option<String>,
}

impl ThreadsApiPost {
    pub fn into_raw_post(self, raw_json: String) -> ThreadPostRaw {
        let text = self.text.or(self.caption).unwrap_or_default();
        let (author_id, owner_username) = self
            .owner
            .map(|owner| (owner.id, owner.username))
            .unwrap_or((None, None));
        let author_username = self.username.or(owner_username);

        ThreadPostRaw {
            post_id: self.id,
            text_missing: text.trim().is_empty(),
            text,
            author_id,
            author_username,
            media_type: self.media_type,
            permalink: self.permalink,
            posted_at: self.timestamp,
            raw_json,
        }
    }
}

impl SampleThreadsPost {
    pub fn into_raw_post(self) -> ThreadPostRaw {
        let raw_json = serde_json::json!({
            "id": &self.post_id,
            "text": &self.text,
            "media_type": &self.media_type,
            "permalink": &self.permalink,
            "timestamp": &self.posted_at,
            "source": "sample_threads_posts"
        })
        .to_string();

        ThreadPostRaw {
            post_id: self.post_id,
            text: self.text,
            text_missing: false,
            author_id: None,
            author_username: None,
            media_type: self.media_type,
            permalink: self.permalink,
            posted_at: self.posted_at,
            raw_json,
        }
    }
}
