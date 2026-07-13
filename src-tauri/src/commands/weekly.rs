use crate::models::trend::WeeklyAggregationResult;
use crate::services::weekly_aggregator;

#[tauri::command]
pub fn aggregate_weekly_metrics() -> Result<WeeklyAggregationResult, String> {
    weekly_aggregator::aggregate_weekly_metrics()
}
