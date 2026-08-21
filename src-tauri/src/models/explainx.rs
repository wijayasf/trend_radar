use serde::Serialize;

#[derive(Debug, Clone)]
pub struct ExplainXRecordInput {
    pub source_record_key: String,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub url: Option<String>,
    pub source_url: Option<String>,
    pub pricing_text: Option<String>,
    pub platform_text: Option<String>,
    pub record_type: String,
    pub raw_json: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExplainXImportPreview {
    pub source_record_key: String,
    pub name: String,
    pub category: String,
    pub tags: Vec<String>,
    pub identity_status: String,
    pub matched_canonical_entity: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExplainXImportResult {
    pub imported: usize,
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub invalid: usize,
    pub linked_exact_alias: usize,
    pub review_needed: usize,
    pub unlinked: usize,
    pub ingestion_batch_id: String,
    pub message: String,
    pub sample_records: Vec<ExplainXImportPreview>,
}
