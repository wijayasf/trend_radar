use serde::{Deserialize, Serialize};

pub const CROSS_SOURCE_SCORE_VERSION: &str = "cross-source-v1-proposal";
pub const TRUSTED_RANKING_LABEL: &str = "trusted_ranking";
pub const WATCHLIST_LABEL: &str = "watchlist";
pub const NEEDS_REVIEW_LABEL: &str = "needs_review";
pub const EXCLUDED_FROM_SCORE_LABEL: &str = "excluded_from_score";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CrossSourceFactorBreakdown {
    pub mention_count_score: f64,
    pub sentiment_score: f64,
    pub cost_signal_score: f64,
    pub region_signal_score: f64,
    pub registry_presence_score: f64,
    pub source_diversity_score: f64,
    pub review_confidence_score: f64,
    pub recency_score: f64,
    pub sentiment_adjustment: f64,
    pub cost_adjustment: f64,
    pub conversation_score: f64,
    pub cross_source_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrossSourceScorePreview {
    pub rank: usize,
    pub score_version: String,
    pub week_start: String,
    pub week_end: String,
    pub entity_id: String,
    pub canonical_name: String,
    pub entity_type: String,
    pub region: String,
    pub mention_count: usize,
    pub approved_registry_record_count: usize,
    pub conversation_source_count: usize,
    pub conversation_score: f64,
    pub registry_score: f64,
    pub source_diversity_score: f64,
    pub review_confidence_score: f64,
    pub recency_score: f64,
    pub cost_adjustment: f64,
    pub sentiment_adjustment: f64,
    pub cross_source_score: f64,
    pub ranking_label: String,
    pub factor_breakdown_json: String,
    pub source_evidence_json: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CrossSourceScoreDiagnostic {
    pub entity_name: String,
    pub entity_type: Option<String>,
    pub region: Option<String>,
    pub ranking_label: String,
    pub reason: String,
    pub source_record_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrossSourceFixtureValidationResult {
    pub fixture_version: String,
    pub score_version: String,
    pub passed: bool,
    pub assertions_checked: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrossSourceScoreAggregationResult {
    pub score_version: String,
    pub week_start: Option<String>,
    pub week_end: Option<String>,
    pub scored_rows: usize,
    pub trusted_ranking_rows: usize,
    pub watchlist_rows: usize,
    pub needs_review_rows: usize,
    pub excluded_rows: usize,
    pub top_global: Vec<CrossSourceScorePreview>,
    pub top_indonesia: Vec<CrossSourceScorePreview>,
    pub factor_breakdown_preview: Vec<CrossSourceScorePreview>,
    pub watchlist: Vec<CrossSourceScoreDiagnostic>,
    pub needs_review: Vec<CrossSourceScoreDiagnostic>,
    pub excluded_from_score: Vec<CrossSourceScoreDiagnostic>,
    pub fixture_validation: Option<CrossSourceFixtureValidationResult>,
    pub message: String,
}
