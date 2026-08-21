use crate::models::trend::{WeeklyAggregationResult, WeeklyEntityAggregationResult};
use crate::services::{canonical_weekly_aggregator, weekly_aggregator};

#[tauri::command]
pub fn aggregate_weekly_metrics() -> Result<WeeklyAggregationResult, String> {
    weekly_aggregator::aggregate_weekly_metrics()
}

#[tauri::command]
pub fn aggregate_weekly_entity_metrics() -> Result<WeeklyEntityAggregationResult, String> {
    canonical_weekly_aggregator::aggregate_weekly_entity_metrics()
}
