use crate::models::cross_source::CrossSourceScoreAggregationResult;
use crate::models::trend::{WeeklyAggregationResult, WeeklyEntityAggregationResult};
use crate::services::{
    canonical_weekly_aggregator, cross_source_score_aggregator, weekly_aggregator,
};

#[tauri::command]
pub fn aggregate_weekly_metrics() -> Result<WeeklyAggregationResult, String> {
    weekly_aggregator::aggregate_weekly_metrics()
}

#[tauri::command]
pub fn aggregate_weekly_entity_metrics() -> Result<WeeklyEntityAggregationResult, String> {
    canonical_weekly_aggregator::aggregate_weekly_entity_metrics()
}

#[tauri::command]
pub fn aggregate_cross_source_entity_scores() -> Result<CrossSourceScoreAggregationResult, String> {
    cross_source_score_aggregator::aggregate_cross_source_entity_scores()
}
