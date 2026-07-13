use crate::models::entities::{
    CandidateEntityListResult, CandidateReviewActionResult, EntityReviewDecisionListResult,
};
use crate::services::candidate_review;

#[tauri::command]
pub fn list_candidate_entities() -> Result<CandidateEntityListResult, String> {
    candidate_review::list_candidate_entities()
}

#[tauri::command]
pub fn list_entity_review_decisions() -> Result<EntityReviewDecisionListResult, String> {
    candidate_review::list_entity_review_decisions()
}

#[tauri::command]
pub fn approve_candidate_entity(
    candidate_name: String,
    reviewed_as: String,
    reviewed_category: String,
    note: Option<String>,
) -> Result<CandidateReviewActionResult, String> {
    candidate_review::approve_candidate_entity(candidate_name, reviewed_as, reviewed_category, note)
}

#[tauri::command]
pub fn ignore_candidate_entity(
    candidate_name: String,
    note: Option<String>,
) -> Result<CandidateReviewActionResult, String> {
    candidate_review::ignore_candidate_entity(candidate_name, note)
}

#[tauri::command]
pub fn reset_candidate_review(
    candidate_name: String,
) -> Result<CandidateReviewActionResult, String> {
    candidate_review::reset_candidate_review(candidate_name)
}
