use crate::models::external_reviews::{
    ExternalIdentityReviewHistoryResult, ExternalIdentityReviewListResult,
    ExternalIdentityReviewSubmissionResult,
};
use crate::services::external_identity_review;

#[tauri::command]
pub fn list_external_identity_review_items() -> Result<ExternalIdentityReviewListResult, String> {
    external_identity_review::list_external_identity_review_items()
}

#[tauri::command]
pub fn submit_external_identity_review(
    link_id: String,
    relationship_type: String,
    decision: String,
    reviewer: String,
    evidence_note: Option<String>,
) -> Result<ExternalIdentityReviewSubmissionResult, String> {
    external_identity_review::submit_external_identity_review(
        link_id,
        relationship_type,
        decision,
        reviewer,
        evidence_note,
    )
}

#[tauri::command]
pub fn get_external_identity_review_history(
    link_id: String,
) -> Result<ExternalIdentityReviewHistoryResult, String> {
    external_identity_review::get_external_identity_review_history(link_id)
}
