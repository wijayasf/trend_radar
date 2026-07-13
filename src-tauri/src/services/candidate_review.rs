use crate::models::entities::{CandidateEntityListResult, CandidateReviewActionResult};
use crate::services::duckdb_service;

pub fn list_candidate_entities() -> Result<CandidateEntityListResult, String> {
    let candidates = duckdb_service::list_candidate_entities()?;
    let pending_count = candidates
        .iter()
        .filter(|candidate| candidate.current_status == "pending")
        .count();
    let approved_count = candidates
        .iter()
        .filter(|candidate| candidate.current_status == "approved")
        .count();
    let ignored_count = candidates
        .iter()
        .filter(|candidate| candidate.current_status == "ignored")
        .count();

    Ok(CandidateEntityListResult {
        total_candidates: candidates.len(),
        pending_count,
        approved_count,
        ignored_count,
        message: format!(
            "Loaded {} candidate entities, including {} pending review.",
            candidates.len(),
            pending_count
        ),
        candidates,
    })
}

pub fn approve_candidate_entity(
    candidate_name: String,
    reviewed_as: String,
    reviewed_category: String,
    note: Option<String>,
) -> Result<CandidateReviewActionResult, String> {
    let updated_mentions_count = duckdb_service::approve_candidate_entity(
        &candidate_name,
        &reviewed_as,
        &reviewed_category,
        note,
    )?;

    Ok(CandidateReviewActionResult {
        candidate_name: candidate_name.clone(),
        status: "approved".to_string(),
        updated_mentions_count,
        message: format!(
            "Approved {candidate_name} as {reviewed_as} and updated {updated_mentions_count} mentions."
        ),
    })
}

pub fn ignore_candidate_entity(
    candidate_name: String,
    note: Option<String>,
) -> Result<CandidateReviewActionResult, String> {
    let updated_mentions_count = duckdb_service::ignore_candidate_entity(&candidate_name, note)?;

    Ok(CandidateReviewActionResult {
        candidate_name: candidate_name.clone(),
        status: "ignored".to_string(),
        updated_mentions_count,
        message: format!("Ignored {candidate_name} and updated {updated_mentions_count} mentions."),
    })
}

pub fn reset_candidate_review(
    candidate_name: String,
) -> Result<CandidateReviewActionResult, String> {
    let updated_mentions_count = duckdb_service::reset_candidate_review(&candidate_name)?;

    Ok(CandidateReviewActionResult {
        candidate_name: candidate_name.clone(),
        status: "pending".to_string(),
        updated_mentions_count,
        message: format!(
            "Reset {candidate_name} to pending review and updated {updated_mentions_count} mentions."
        ),
    })
}
