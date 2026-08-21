use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ExternalIdentityReviewItem {
    pub link_id: String,
    pub source: String,
    pub source_record_id: String,
    pub source_record_key: String,
    pub source_record_name: String,
    pub source_record_url: Option<String>,
    pub source_record_description: Option<String>,
    pub canonical_entity_id: String,
    pub canonical_entity_name: String,
    pub canonical_entity_type: String,
    pub relationship_type: String,
    pub current_status: String,
    pub match_method: String,
    pub match_confidence: Option<f64>,
    pub match_reason: String,
    pub evidence: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub latest_decision: Option<String>,
    pub latest_reviewer: Option<String>,
    pub latest_review_note: Option<String>,
    pub latest_reviewed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalIdentityReviewListResult {
    pub total: usize,
    pub pending_count: usize,
    pub approved_count: usize,
    pub rejected_count: usize,
    pub ambiguous_count: usize,
    pub items: Vec<ExternalIdentityReviewItem>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalIdentityReviewHistoryEntry {
    pub review_id: String,
    pub link_id: String,
    pub source_record_id: String,
    pub canonical_entity_id: String,
    pub previous_state: String,
    pub decision: String,
    pub proposed_relationship_type: String,
    pub match_method: String,
    pub match_confidence: Option<f64>,
    pub evidence: Option<String>,
    pub review_note: Option<String>,
    pub reviewer: String,
    pub reviewed_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalIdentityReviewHistoryResult {
    pub link_id: String,
    pub total: usize,
    pub history: Vec<ExternalIdentityReviewHistoryEntry>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalIdentityReviewSubmissionResult {
    pub item: ExternalIdentityReviewItem,
    pub history_count: usize,
    pub message: String,
}
