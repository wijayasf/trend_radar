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
pub struct DetectedAgentMention {
    pub mention_id: String,
    pub post_id: String,
    pub agent_name: String,
    pub agent_alias: String,
    pub category: String,
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
    pub region: String,
    pub region_confidence: f64,
    pub region_reason: String,
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

impl From<&DetectedAgentMention> for AgentMentionPreview {
    fn from(mention: &DetectedAgentMention) -> Self {
        Self {
            agent_name: mention.agent_name.clone(),
            category: mention.category.clone(),
            region: mention.region.clone(),
            region_confidence: 0.0,
            region_reason: "not classified".to_string(),
            confidence: mention.match_confidence,
            source_snippet: mention.source_snippet.clone(),
        }
    }
}
