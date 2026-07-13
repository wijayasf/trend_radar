use crate::models::entities::CostClassificationResult;
use crate::services::cost_classifier;

#[tauri::command]
pub fn classify_cost_signals() -> Result<CostClassificationResult, String> {
    cost_classifier::classify_cost_signals()
}
