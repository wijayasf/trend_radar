use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sentiment {
    Positive,
    Negative,
    Neutral,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CostSignal {
    Expensive,
    TokenHeavy,
    QuotaLimited,
    WorthIt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrendTopic {
    pub name: String,
    pub region: TrendRegion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrendRegion {
    Indonesia,
    Global,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeeklyAgentMetric {
    pub rank: usize,
    pub week_start: String,
    pub week_end: String,
    pub region: String,
    pub agent_name: String,
    pub category: String,
    pub mentions: usize,
    pub positive_count: usize,
    pub neutral_count: usize,
    pub negative_count: usize,
    pub mixed_count: usize,
    pub cost_not_mentioned_count: usize,
    pub cost_positive_count: usize,
    pub cost_negative_boros_count: usize,
    pub cost_mixed_count: usize,
    pub positive_pct: f64,
    pub negative_pct: f64,
    pub cost_negative_boros_pct: f64,
    pub trend_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeeklyAggregationResult {
    pub metrics_count: usize,
    pub indonesia_count: usize,
    pub global_count: usize,
    pub unknown_count: usize,
    pub top_indonesia: Vec<WeeklyAgentMetric>,
    pub top_global: Vec<WeeklyAgentMetric>,
    pub top_unknown: Vec<WeeklyAgentMetric>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeeklyEntityMetric {
    pub rank: usize,
    pub id: String,
    pub week_start: String,
    pub week_end: String,
    pub entity_id: String,
    pub canonical_name: String,
    pub entity_type: String,
    pub region: String,
    pub mention_count: usize,
    pub positive_count: usize,
    pub neutral_count: usize,
    pub negative_count: usize,
    pub mixed_count: usize,
    pub cost_positive_count: usize,
    pub cost_negative_boros_count: usize,
    pub cost_mixed_count: usize,
    pub cost_not_mentioned_count: usize,
    pub source_count: usize,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub trend_score: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentityResolutionSkipCounts {
    pub unresolved: usize,
    pub ambiguous: usize,
    pub missing_alias: usize,
    pub skipped: usize,
    pub invalid_resolved: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeeklyEntityAggregationResult {
    pub canonical_rows_generated: usize,
    pub resolved_entities_included: usize,
    pub unresolved_mentions_skipped: usize,
    pub ambiguous_mentions_skipped: usize,
    pub missing_alias_mentions_skipped: usize,
    pub skipped_mentions_skipped: usize,
    pub top_indonesia: Vec<WeeklyEntityMetric>,
    pub top_global: Vec<WeeklyEntityMetric>,
    pub errors: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportExportResult {
    pub file_path: String,
    pub rows_exported: usize,
    pub message: String,
    pub preview: String,
}
