use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct AliasesConfig {
    #[serde(default)]
    pub agents: Vec<AgentAliasConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentAliasConfig {
    pub canonical_name: String,
    pub category: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub ambiguous: bool,
    #[serde(default)]
    pub context_terms: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RawPostForDetection {
    pub post_id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct RegionClassification {
    pub post_id: String,
    pub region: String,
    pub region_confidence: f64,
    pub region_reason: String,
}

#[derive(Debug, Clone)]
pub struct AgentMentionForSentiment {
    pub mention_id: String,
    pub source_snippet: String,
}

#[derive(Debug, Clone)]
pub struct SentimentClassification {
    pub mention_id: String,
    pub sentiment: String,
    pub sentiment_confidence: f64,
    pub sentiment_reason: String,
}

#[derive(Debug, Clone)]
pub struct AgentMentionForCost {
    pub mention_id: String,
    pub source_snippet: String,
}

#[derive(Debug, Clone)]
pub struct CostClassification {
    pub mention_id: String,
    pub cost_signal: String,
    pub cost_confidence: f64,
    pub cost_reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidateEntityReview {
    pub candidate_name: String,
    pub mention_count: usize,
    pub first_seen: String,
    pub latest_seen: String,
    pub sample_snippets: Vec<String>,
    pub current_status: String,
    pub reviewed_as: String,
    pub reviewed_category: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityReviewDecision {
    pub id: String,
    pub candidate_name: String,
    pub normalized_name: String,
    pub category: String,
    pub status: String,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityReviewDecisionListResult {
    pub total_decisions: usize,
    pub approved_count: usize,
    pub ignored_count: usize,
    pub decisions: Vec<EntityReviewDecision>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidateEntityListResult {
    pub total_candidates: usize,
    pub pending_count: usize,
    pub approved_count: usize,
    pub ignored_count: usize,
    pub candidates: Vec<CandidateEntityReview>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidateReviewActionResult {
    pub candidate_name: String,
    pub status: String,
    pub updated_mentions_count: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct DetectedAgentMention {
    pub mention_id: String,
    pub post_id: String,
    pub agent_name: String,
    pub agent_alias: String,
    pub category: String,
    pub detection_source: String,
    pub needs_review: bool,
    pub review_status: String,
    pub reviewed_as: Option<String>,
    pub reviewed_category: Option<String>,
    pub region: String,
    pub confidence: f64,
    pub match_confidence: f64,
    pub relevance_score: f64,
    pub sentiment: String,
    pub cost_signal: String,
    pub source_snippet: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentMentionPreview {
    pub agent_name: String,
    pub category: String,
    pub detection_source: String,
    pub needs_review: bool,
    pub region: String,
    pub region_confidence: f64,
    pub region_reason: String,
    pub sentiment: String,
    pub sentiment_confidence: f64,
    pub sentiment_reason: String,
    pub cost_signal: String,
    pub cost_confidence: f64,
    pub cost_reason: String,
    pub confidence: f64,
    pub source_snippet: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityDetectionResult {
    pub analyzed_posts: usize,
    pub mentions_found: usize,
    pub saved_count: usize,
    pub message: String,
    pub preview: Vec<AgentMentionPreview>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegionClassificationResult {
    pub posts_analyzed: usize,
    pub indonesia_count: usize,
    pub global_count: usize,
    pub unknown_count: usize,
    pub updated_mentions_count: usize,
    pub message: String,
    pub preview: Vec<AgentMentionPreview>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SentimentClassificationResult {
    pub mentions_analyzed: usize,
    pub positive_count: usize,
    pub neutral_count: usize,
    pub negative_count: usize,
    pub mixed_count: usize,
    pub updated_mentions_count: usize,
    pub message: String,
    pub preview: Vec<AgentMentionPreview>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostClassificationResult {
    pub mentions_analyzed: usize,
    pub not_mentioned_count: usize,
    pub cost_positive_count: usize,
    pub cost_negative_boros_count: usize,
    pub cost_mixed_count: usize,
    pub updated_mentions_count: usize,
    pub message: String,
    pub preview: Vec<AgentMentionPreview>,
}

impl From<&DetectedAgentMention> for AgentMentionPreview {
    fn from(mention: &DetectedAgentMention) -> Self {
        Self {
            agent_name: mention.agent_name.clone(),
            category: mention.category.clone(),
            detection_source: mention.detection_source.clone(),
            needs_review: mention.needs_review,
            region: mention.region.clone(),
            region_confidence: 0.0,
            region_reason: "not classified".to_string(),
            sentiment: mention.sentiment.clone(),
            sentiment_confidence: 0.0,
            sentiment_reason: "not classified".to_string(),
            cost_signal: mention.cost_signal.clone(),
            cost_confidence: 0.0,
            cost_reason: "not classified".to_string(),
            confidence: mention.match_confidence,
            source_snippet: mention.source_snippet.clone(),
        }
    }
}
