use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ThreadPostRaw {
    pub post_id: String,
    pub text: String,
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
        ThreadPostRaw {
            post_id: self.id,
            text: self.text.or(self.caption).unwrap_or_default(),
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
            media_type: self.media_type,
            permalink: self.permalink,
            posted_at: self.posted_at,
            raw_json,
        }
    }
}
