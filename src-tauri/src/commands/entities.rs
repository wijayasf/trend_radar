use crate::models::entities::{EntityDetectionResult, MentionIdentityLinkageResult};
use crate::services::{entity_detector, identity_linker};

#[tauri::command]
pub fn detect_agent_mentions() -> Result<EntityDetectionResult, String> {
    entity_detector::detect_agent_mentions()
}

#[tauri::command]
pub fn link_agent_mentions_to_entities() -> Result<MentionIdentityLinkageResult, String> {
    identity_linker::link_agent_mentions_to_entities()
}
