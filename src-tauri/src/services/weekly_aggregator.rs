use crate::models::trend::WeeklyAggregationResult;
use crate::services::duckdb_service;

const REGION_LIMIT: usize = 20;

pub fn aggregate_weekly_metrics() -> Result<WeeklyAggregationResult, String> {
    let metrics_count = duckdb_service::rebuild_weekly_agent_metrics()?;
    let top_indonesia =
        duckdb_service::load_weekly_agent_metrics_by_region("indonesia", REGION_LIMIT)?;
    let top_global = duckdb_service::load_weekly_agent_metrics_by_region("global", REGION_LIMIT)?;
    let top_unknown = duckdb_service::load_weekly_agent_metrics_by_region("unknown", REGION_LIMIT)?;

    Ok(WeeklyAggregationResult {
        metrics_count,
        indonesia_count: top_indonesia.len(),
        global_count: top_global.len(),
        unknown_count: top_unknown.len(),
        message: format!("Aggregated {metrics_count} weekly agent metric rows."),
        top_indonesia,
        top_global,
        top_unknown,
    })
}
