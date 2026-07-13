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
pub struct ReportExportResult {
    pub file_path: String,
    pub rows_exported: usize,
    pub message: String,
    pub preview: String,
}
