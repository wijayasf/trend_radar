use crate::models::entities::EntityDetectionResult;
use crate::services::entity_detector;

#[tauri::command]
pub fn detect_agent_mentions() -> Result<EntityDetectionResult, String> {
    entity_detector::detect_agent_mentions()
}
