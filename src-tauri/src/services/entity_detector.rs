use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use crate::models::entities::{
    AgentAliasConfig, AliasesConfig, DetectedAgentMention, EntityDetectionResult,
    EntityReviewDecision, NamedEntityGateMatch,
};
use crate::services::duckdb_service;

const ALIASES_CONFIG_PATH: &str = "config/aliases.yml";
const PREVIEW_LIMIT: usize = 12;
const SNIPPET_LIMIT: usize = 180;

pub struct NamedEntityGateDetector {
    config: AliasesConfig,
}

impl NamedEntityGateDetector {
    pub fn load() -> Result<Self, String> {
        Ok(Self {
            config: load_aliases_config()?,
        })
    }

    pub fn detect(&self, text: &str) -> Vec<NamedEntityGateMatch> {
        let normalized_text = normalize_text(text);
        let mentions =
            detect_mentions_in_text("named-entity-gate", text, &self.config, &HashMap::new());
        let has_other_concrete_entity = mentions.iter().any(|mention| {
            !is_gate_context_only_entity(&mention.agent_name)
                && !is_gate_ambiguous_entity(&mention.agent_name)
        });

        mentions
            .into_iter()
            .filter(|mention| !is_gate_context_only_entity(&mention.agent_name))
            .filter(|mention| {
                !is_gate_ambiguous_entity(&mention.agent_name)
                    || has_other_concrete_entity
                    || has_candidate_context(&normalized_text)
            })
            .map(|mention| NamedEntityGateMatch {
                entity_name: mention.agent_name,
                category: mention.category,
                detection_source: mention.detection_source,
            })
            .collect()
    }
}

pub fn detect_agent_mentions() -> Result<EntityDetectionResult, String> {
    let config = load_aliases_config()?;
    if config.agents.is_empty() {
        return Err("No agent aliases configured in config/aliases.yml".to_string());
    }

    let posts = duckdb_service::load_raw_posts_for_detection()?;
    let decisions = entity_review_decisions_by_id()?;
    let mut mentions = Vec::new();

    for post in &posts {
        mentions.extend(detect_mentions_in_text(
            &post.post_id,
            &post.text,
            &config,
            &decisions,
        ));
    }

    let saved_count = duckdb_service::save_agent_mentions(&mentions)?;
    let preview = mentions
        .iter()
        .take(PREVIEW_LIMIT)
        .map(Into::into)
        .collect();

    Ok(EntityDetectionResult {
        analyzed_posts: posts.len(),
        mentions_found: mentions.len(),
        saved_count,
        message: format!(
            "Analyzed {} raw posts and saved {} agent mentions.",
            posts.len(),
            saved_count
        ),
        preview,
    })
}

fn load_aliases_config() -> Result<AliasesConfig, String> {
    let config_path = find_aliases_config_path().ok_or_else(|| {
        format!("Could not find {ALIASES_CONFIG_PATH} from the app working directory")
    })?;
    let config_text = fs::read_to_string(&config_path).map_err(|error| {
        format!(
            "Failed to read aliases config at {}: {error}",
            config_path.display()
        )
    })?;

    serde_yaml::from_str(&config_text).map_err(|error| {
        format!(
            "Failed to parse aliases config at {}: {error}",
            config_path.display()
        )
    })
}

fn find_aliases_config_path() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        PathBuf::from(ALIASES_CONFIG_PATH),
        PathBuf::from("..").join(ALIASES_CONFIG_PATH),
        manifest_dir.join("..").join(ALIASES_CONFIG_PATH),
    ];

    candidates.into_iter().find(|candidate| candidate.exists())
}

fn detect_mentions_in_text(
    post_id: &str,
    text: &str,
    config: &AliasesConfig,
    decisions: &HashMap<String, EntityReviewDecision>,
) -> Vec<DetectedAgentMention> {
    let normalized_text = normalize_text(text);
    if normalized_text.is_empty() {
        return Vec::new();
    }

    let mut seen_agents = HashSet::new();
    let mut mentions = Vec::new();
    let known_aliases = known_aliases(config);
    let mut matched_known_aliases = Vec::new();

    for agent in &config.agents {
        if seen_agents.contains(&agent.canonical_name) {
            continue;
        }

        if let Some((matched_alias, confidence)) = detect_agent_alias(agent, &normalized_text) {
            seen_agents.insert(agent.canonical_name.clone());
            matched_known_aliases.push(normalize_text(&matched_alias));
            mentions.push(DetectedAgentMention {
                mention_id: stable_mention_id(post_id, &agent.canonical_name),
                post_id: post_id.to_string(),
                agent_name: agent.canonical_name.clone(),
                agent_alias: matched_alias.clone(),
                category: agent.category.clone(),
                detection_source: "known_alias".to_string(),
                needs_review: false,
                review_status: "approved".to_string(),
                reviewed_as: None,
                reviewed_category: None,
                region: "unknown".to_string(),
                confidence,
                match_confidence: confidence,
                relevance_score: relevance_score(confidence, &normalized_text),
                sentiment: "unknown".to_string(),
                cost_signal: "none".to_string(),
                source_snippet: source_snippet(text),
            });
        }
    }

    mentions.extend(detect_candidate_mentions(
        post_id,
        text,
        &normalized_text,
        &known_aliases,
        &matched_known_aliases,
        decisions,
        &mut seen_agents,
    ));

    mentions
}

fn detect_agent_alias(agent: &AgentAliasConfig, normalized_text: &str) -> Option<(String, f64)> {
    let mut candidates = Vec::with_capacity(agent.aliases.len() + 1);
    candidates.push(agent.canonical_name.as_str());
    candidates.extend(agent.aliases.iter().map(String::as_str));
    candidates.sort_by_key(|candidate| std::cmp::Reverse(normalize_text(candidate).len()));
    candidates.dedup_by(|left, right| normalize_text(left) == normalize_text(right));

    for candidate in candidates {
        let normalized_alias = normalize_text(candidate);
        if normalized_alias.is_empty() || !contains_alias(normalized_text, &normalized_alias) {
            continue;
        }

        if agent.ambiguous && !has_required_context(agent, normalized_text, &normalized_alias) {
            continue;
        }

        return Some((
            candidate.to_string(),
            confidence_for(agent, &normalized_alias),
        ));
    }

    None
}

fn contains_alias(normalized_text: &str, normalized_alias: &str) -> bool {
    let searchable_text = format!(" {normalized_text} ");
    let searchable_alias = format!(" {normalized_alias} ");
    searchable_text.contains(&searchable_alias)
}

fn has_required_context(
    agent: &AgentAliasConfig,
    normalized_text: &str,
    normalized_alias: &str,
) -> bool {
    let context = context_window(normalized_text, normalized_alias);
    agent
        .context_terms
        .iter()
        .map(|term| normalize_text(term))
        .any(|term| !term.is_empty() && contains_alias(&context, &term))
}

fn context_window(normalized_text: &str, normalized_alias: &str) -> String {
    let text_tokens: Vec<&str> = normalized_text.split_whitespace().collect();
    let alias_tokens: Vec<&str> = normalized_alias.split_whitespace().collect();

    if alias_tokens.is_empty() || text_tokens.is_empty() || alias_tokens.len() > text_tokens.len() {
        return normalized_text.to_string();
    }

    for start in 0..=text_tokens.len() - alias_tokens.len() {
        if text_tokens[start..start + alias_tokens.len()] == alias_tokens {
            let window_start = start.saturating_sub(10);
            let window_end = (start + alias_tokens.len() + 10).min(text_tokens.len());
            return text_tokens[window_start..window_end].join(" ");
        }
    }

    normalized_text.to_string()
}

fn confidence_for(agent: &AgentAliasConfig, normalized_alias: &str) -> f64 {
    let normalized_canonical = normalize_text(&agent.canonical_name);
    if agent.ambiguous {
        if normalized_alias == normalized_canonical {
            0.76
        } else {
            0.88
        }
    } else if normalized_alias == normalized_canonical {
        0.96
    } else if normalized_alias.split_whitespace().count() > 1 {
        0.92
    } else {
        0.86
    }
}

fn relevance_score(confidence: f64, normalized_text: &str) -> f64 {
    let has_agent_context = candidate_context_terms()
        .iter()
        .any(|term| contains_alias(normalized_text, term));

    if has_agent_context {
        (confidence + 0.04_f64).min(1.0)
    } else {
        confidence
    }
}

fn detect_candidate_mentions(
    post_id: &str,
    text: &str,
    normalized_text: &str,
    known_aliases: &HashSet<String>,
    matched_known_aliases: &[String],
    decisions: &HashMap<String, EntityReviewDecision>,
    seen_agents: &mut HashSet<String>,
) -> Vec<DetectedAgentMention> {
    if !has_candidate_context(normalized_text) {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    for candidate in extract_candidate_names(text) {
        let normalized_candidate = normalize_text(&candidate);
        if normalized_candidate.is_empty()
            || is_candidate_stop_phrase(&normalized_candidate)
            || !is_meaningful_unknown_candidate(&candidate, &normalized_candidate, normalized_text)
            || known_aliases.contains(&normalized_candidate)
            || overlaps_known_alias_fragment(
                &normalized_candidate,
                matched_known_aliases,
                known_aliases,
            )
            || (!matched_known_aliases.is_empty()
                && !is_strong_unknown_candidate(&candidate, &normalized_candidate, normalized_text))
            || seen_agents.iter().any(|agent| {
                let normalized_agent = normalize_text(agent);
                normalized_agent == normalized_candidate
                    || normalized_candidate.starts_with(&format!("{normalized_agent} "))
                    || normalized_candidate.ends_with(&format!(" {normalized_agent}"))
            })
        {
            continue;
        }

        seen_agents.insert(candidate.clone());
        let decision = decisions.get(&normalized_candidate);
        candidates.push(candidate_mention_from_decision(
            post_id, text, candidate, decision,
        ));
    }

    candidates
}

fn overlaps_known_alias_fragment(
    normalized_candidate: &str,
    matched_known_aliases: &[String],
    known_aliases: &HashSet<String>,
) -> bool {
    let candidate_tokens = normalized_candidate.split_whitespace().collect::<Vec<_>>();
    if candidate_tokens.is_empty() {
        return false;
    }

    let overlaps = |alias: &str| {
        let alias_tokens = alias.split_whitespace().collect::<Vec<_>>();
        candidate_tokens.len() < alias_tokens.len()
            && alias_tokens
                .windows(candidate_tokens.len())
                .any(|window| window == candidate_tokens)
    };

    matched_known_aliases.iter().any(|alias| overlaps(alias))
        || (candidate_tokens.len() == 1 && known_aliases.iter().any(|alias| overlaps(alias)))
}

fn entity_review_decisions_by_id() -> Result<HashMap<String, EntityReviewDecision>, String> {
    let mut decisions = HashMap::new();
    for decision in duckdb_service::load_entity_review_decisions()? {
        decisions.insert(decision.id.clone(), decision);
    }
    Ok(decisions)
}

fn candidate_mention_from_decision(
    post_id: &str,
    text: &str,
    candidate: String,
    decision: Option<&EntityReviewDecision>,
) -> DetectedAgentMention {
    let mention_id = stable_mention_id(post_id, &format!("candidate::{candidate}"));

    if let Some(decision) = decision {
        if decision.status == "approved" {
            return DetectedAgentMention {
                mention_id,
                post_id: post_id.to_string(),
                agent_name: decision.normalized_name.clone(),
                agent_alias: candidate,
                category: decision.category.clone(),
                detection_source: "reviewed_candidate".to_string(),
                needs_review: false,
                review_status: "approved".to_string(),
                reviewed_as: Some(decision.normalized_name.clone()),
                reviewed_category: Some(decision.category.clone()),
                region: "unknown".to_string(),
                confidence: 0.82,
                match_confidence: 0.82,
                relevance_score: 0.86,
                sentiment: "unknown".to_string(),
                cost_signal: "none".to_string(),
                source_snippet: source_snippet(text),
            };
        }

        if decision.status == "ignored" {
            return DetectedAgentMention {
                mention_id,
                post_id: post_id.to_string(),
                agent_name: candidate.clone(),
                agent_alias: candidate,
                category: "unknown_candidate".to_string(),
                detection_source: "candidate_pattern".to_string(),
                needs_review: false,
                review_status: "ignored".to_string(),
                reviewed_as: None,
                reviewed_category: None,
                region: "unknown".to_string(),
                confidence: 0.62,
                match_confidence: 0.62,
                relevance_score: 0.66,
                sentiment: "unknown".to_string(),
                cost_signal: "none".to_string(),
                source_snippet: source_snippet(text),
            };
        }
    }

    DetectedAgentMention {
        mention_id,
        post_id: post_id.to_string(),
        agent_name: candidate.clone(),
        agent_alias: candidate,
        category: "unknown_candidate".to_string(),
        detection_source: "candidate_pattern".to_string(),
        needs_review: true,
        review_status: "pending".to_string(),
        reviewed_as: None,
        reviewed_category: None,
        region: "unknown".to_string(),
        confidence: 0.62,
        match_confidence: 0.62,
        relevance_score: 0.66,
        sentiment: "unknown".to_string(),
        cost_signal: "none".to_string(),
        source_snippet: source_snippet(text),
    }
}

fn known_aliases(config: &AliasesConfig) -> HashSet<String> {
    let mut aliases = HashSet::new();
    for agent in &config.agents {
        let canonical = normalize_text(&agent.canonical_name);
        if !canonical.is_empty() {
            aliases.insert(canonical);
        }
        for alias in &agent.aliases {
            let normalized_alias = normalize_text(alias);
            if !normalized_alias.is_empty() {
                aliases.insert(normalized_alias);
            }
        }
    }
    aliases
}

fn is_gate_context_only_entity(entity_name: &str) -> bool {
    matches!(normalize_text(entity_name).as_str(), "mcp")
}

fn is_gate_ambiguous_entity(entity_name: &str) -> bool {
    matches!(normalize_text(entity_name).as_str(), "ponytail" | "caveman")
}

fn has_candidate_context(normalized_text: &str) -> bool {
    candidate_context_terms()
        .iter()
        .any(|term| contains_alias(normalized_text, term))
}

fn candidate_context_terms() -> &'static [&'static str] {
    &[
        "ai",
        "agent",
        "agents",
        "agentic",
        "skill",
        "skills",
        "mcp",
        "mcp server",
        "plugin",
        "tool",
        "tools",
        "framework",
        "workflow",
        "workflows",
        "automation",
        "coding",
        "code",
        "developer",
        "memory",
        "token",
        "server",
    ]
}

fn extract_candidate_names(text: &str) -> Vec<String> {
    let tokens = text
        .split_whitespace()
        .map(clean_candidate_token)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut candidates = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        let token = &tokens[index];
        if is_domain_like(token) {
            candidates.push(token.to_string());
            index += 1;
            continue;
        }

        if !is_capitalized_candidate_token(token) {
            index += 1;
            continue;
        }

        let mut phrase = vec![token.to_string()];
        let mut next = index + 1;
        while next < tokens.len()
            && phrase.len() < 3
            && is_capitalized_candidate_token(tokens[next])
        {
            phrase.push(tokens[next].to_string());
            next += 1;
        }

        if phrase.len() > 1 {
            candidates.extend(phrase.iter().cloned());
        }
        candidates.push(phrase.join(" "));
        index = next;
    }

    candidates
}

fn clean_candidate_token(token: &str) -> &str {
    token.trim_matches(|character: char| !character.is_alphanumeric())
}

fn is_domain_like(token: &str) -> bool {
    let lowercase = token.to_lowercase();
    lowercase.contains('.')
        && lowercase.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '.' || character == '-'
        })
        && lowercase.split('.').all(|part| !part.is_empty())
}

fn is_capitalized_candidate_token(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    first.is_uppercase()
        && token.chars().any(|character| character.is_alphabetic())
        && token.chars().count() >= 3
}

fn is_candidate_stop_phrase(normalized_candidate: &str) -> bool {
    if candidate_stopwords().contains(&normalized_candidate) {
        return true;
    }

    if normalized_candidate
        .split_whitespace()
        .all(|token| candidate_stopwords().contains(&token))
    {
        return true;
    }

    let stop_phrases = [
        "ada",
        "agent",
        "agent come",
        "agentic",
        "agentic ai",
        "ai",
        "ai agent",
        "ai agents",
        "agent trend radar",
        "agent engineer",
        "ai agent trend",
        "ai agent engineer",
        "ai coding",
        "api",
        "apis",
        "appointment setter",
        "appointment setter closer",
        "breaking claude",
        "cli",
        "closer",
        "code",
        "copilot",
        "copilots",
        "developer",
        "entire",
        "genai",
        "genx ers",
        "github",
        "html",
        "indonesia",
        "instagram",
        "large action models",
        "large language models",
        "lam",
        "lams",
        "llm",
        "llms",
        "mcp",
        "mcp server",
        "model context protocol",
        "plugin",
        "same breath",
        "sdk",
        "setter",
        "setter closer",
        "skill",
        "testing",
        "tiktok",
        "threads",
        "threads tiktok instagram",
        "tools",
        "tools ai",
        "trend radar",
        "youtube",
    ];

    stop_phrases.contains(&normalized_candidate)
}

fn is_meaningful_unknown_candidate(
    candidate: &str,
    normalized_candidate: &str,
    normalized_text: &str,
) -> bool {
    let token_count = normalized_candidate.split_whitespace().count();
    if token_count == 0 || token_count > 3 {
        return false;
    }

    if !candidate_appears_near_context(normalized_text, normalized_candidate) {
        return false;
    }

    if unknown_candidate_allowlist().contains(&normalized_candidate) {
        return true;
    }

    if candidate.split_whitespace().any(is_domain_like) {
        return has_domain_product_evidence(normalized_text, normalized_candidate);
    }

    if token_count == 1 {
        return candidate
            .split_whitespace()
            .any(|token| is_camel_case_token(token) || has_product_name_affix(token));
    }

    looks_like_product_or_tool_phrase(candidate, normalized_candidate)
}

fn is_strong_unknown_candidate(
    candidate: &str,
    normalized_candidate: &str,
    normalized_text: &str,
) -> bool {
    if is_candidate_stop_phrase(normalized_candidate) {
        return false;
    }

    if candidate.split_whitespace().any(is_domain_like) {
        return has_domain_product_evidence(normalized_text, normalized_candidate);
    }

    unknown_candidate_allowlist().contains(&normalized_candidate)
        || candidate
            .split_whitespace()
            .any(|token| is_camel_case_token(token) || has_product_name_affix(token))
}

fn has_domain_product_evidence(normalized_text: &str, normalized_candidate: &str) -> bool {
    let context = context_window(normalized_text, normalized_candidate);
    let has_identity_signal = [
        "introducing",
        "introduce",
        "announcing",
        "announce",
        "launched",
        "launching",
        "launch",
        "meet",
        "built",
        "released",
    ]
    .iter()
    .any(|term| contains_alias(&context, term));
    let has_product_signal = [
        "ai agent",
        "agent tool",
        "ai tool",
        "tool",
        "app",
        "assistant",
        "framework",
        "platform",
    ]
    .iter()
    .any(|term| contains_alias(&context, term));

    has_identity_signal && has_product_signal
}

fn looks_like_product_or_tool_phrase(candidate: &str, normalized_candidate: &str) -> bool {
    if normalized_candidate
        .split_whitespace()
        .any(|token| candidate_stopwords().contains(&token))
    {
        return false;
    }

    candidate.split_whitespace().any(|token| {
        is_domain_like(token)
            || is_acronym_token(token)
            || is_camel_case_token(token)
            || has_product_name_affix(token)
    })
}

fn candidate_appears_near_context(normalized_text: &str, normalized_candidate: &str) -> bool {
    let context = context_window(normalized_text, normalized_candidate);
    candidate_near_context_terms()
        .iter()
        .any(|term| contains_alias(&context, term))
}

fn is_acronym_token(token: &str) -> bool {
    let letters = token
        .chars()
        .filter(|character| character.is_alphabetic())
        .collect::<Vec<_>>();

    letters.len() >= 2 && letters.iter().all(|character| character.is_uppercase())
}

fn is_camel_case_token(token: &str) -> bool {
    let letters = token
        .chars()
        .filter(|character| character.is_alphabetic())
        .collect::<Vec<_>>();

    letters.len() >= 4
        && letters.iter().any(|character| character.is_uppercase())
        && letters
            .iter()
            .skip(1)
            .any(|character| character.is_uppercase())
        && letters.iter().any(|character| character.is_lowercase())
}

fn has_product_name_affix(token: &str) -> bool {
    let normalized = token.to_ascii_lowercase();
    candidate_toolish_tokens()
        .iter()
        .any(|affix| normalized.starts_with(affix) || normalized.ends_with(affix))
}

fn candidate_stopwords() -> &'static [&'static str] {
    &[
        "a",
        "an",
        "actually",
        "agentic",
        "and",
        "any",
        "api",
        "apis",
        "appointment",
        "engineer",
        "breaking",
        "but",
        "can",
        "claude",
        "cli",
        "closer",
        "code",
        "codex",
        "copilot",
        "copilots",
        "did",
        "don t",
        "dont",
        "entire",
        "even",
        "everyone",
        "for",
        "genai",
        "genx ers",
        "github",
        "good",
        "he",
        "here",
        "here s",
        "heres",
        "how",
        "html",
        "i",
        "if",
        "i m",
        "im",
        "instagram",
        "it",
        "it s",
        "its",
        "lam",
        "lams",
        "large",
        "llm",
        "llms",
        "mcp",
        "me",
        "models",
        "my",
        "one",
        "plugin",
        "same",
        "save",
        "sdk",
        "setter",
        "she",
        "skill",
        "that",
        "the",
        "they",
        "this",
        "threads",
        "tiktok",
        "to",
        "we",
        "what",
        "when",
        "who",
        "why",
        "with",
        "you",
        "your",
        "youtube",
    ]
}

fn candidate_toolish_tokens() -> &'static [&'static str] {
    &[
        "ai",
        "agent",
        "code",
        "cli",
        "sdk",
        "mcp",
        "graph",
        "studio",
        "labs",
        "copilot",
        "assistant",
        "framework",
        "server",
        "plugin",
        "flow",
    ]
}

fn unknown_candidate_allowlist() -> &'static [&'static str] {
    &["graphify", "headroom"]
}

fn candidate_near_context_terms() -> &'static [&'static str] {
    &[
        "tool",
        "tools",
        "app",
        "framework",
        "plugin",
        "skill",
        "model",
        "mcp",
        "server",
        "agent",
        "agents",
        "workflow",
        "automation",
        "coding",
        "developer",
        "memory",
        "token",
    ]
}

fn stable_mention_id(post_id: &str, agent_name: &str) -> String {
    let slug = normalize_text(agent_name).replace(' ', "_");
    format!("{post_id}::{slug}")
}

fn source_snippet(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= SNIPPET_LIMIT {
        return trimmed.to_string();
    }

    let mut snippet: String = trimmed.chars().take(SNIPPET_LIMIT).collect();
    snippet.push_str("...");
    snippet
}

fn normalize_text(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut previous_was_space = true;

    for character in text.chars() {
        if character.is_alphanumeric() {
            for lowercase in character.to_lowercase() {
                normalized.push(lowercase);
            }
            previous_was_space = false;
        } else if !previous_was_space {
            normalized.push(' ');
            previous_was_space = true;
        }
    }

    normalized.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AliasesConfig {
        AliasesConfig {
            agents: vec![
                AgentAliasConfig {
                    canonical_name: "Claude Code".to_string(),
                    category: "coding_agent".to_string(),
                    aliases: vec!["Claude Code".to_string(), "ClaudeCode".to_string()],
                    ambiguous: false,
                    context_terms: Vec::new(),
                },
                AgentAliasConfig {
                    canonical_name: "Caveman".to_string(),
                    category: "skill_or_mode".to_string(),
                    aliases: vec![
                        "Caveman".to_string(),
                        "Cavemen".to_string(),
                        "Caveman mode".to_string(),
                        "Cavemen mode".to_string(),
                    ],
                    ambiguous: false,
                    context_terms: Vec::new(),
                },
                AgentAliasConfig {
                    canonical_name: "Ponytail".to_string(),
                    category: "skill_or_mode".to_string(),
                    aliases: vec![
                        "Ponytail".to_string(),
                        "Ponytail mode".to_string(),
                        "ponytail.dev".to_string(),
                    ],
                    ambiguous: false,
                    context_terms: Vec::new(),
                },
                AgentAliasConfig {
                    canonical_name: "Astryx".to_string(),
                    category: "coding_agent".to_string(),
                    aliases: vec!["Astryx".to_string(), "astryx.ai".to_string()],
                    ambiguous: false,
                    context_terms: Vec::new(),
                },
                AgentAliasConfig {
                    canonical_name: "ExplainX".to_string(),
                    category: "registry_or_discovery".to_string(),
                    aliases: vec![
                        "ExplainX".to_string(),
                        "Explain X".to_string(),
                        "explainx.ai".to_string(),
                    ],
                    ambiguous: false,
                    context_terms: Vec::new(),
                },
                AgentAliasConfig {
                    canonical_name: "Codex CLI".to_string(),
                    category: "coding_agent".to_string(),
                    aliases: vec!["Codex CLI".to_string(), "OpenAI Codex CLI".to_string()],
                    ambiguous: false,
                    context_terms: Vec::new(),
                },
                AgentAliasConfig {
                    canonical_name: "GitHub Copilot".to_string(),
                    category: "coding_assistant".to_string(),
                    aliases: vec!["GitHub Copilot".to_string(), "Copilot".to_string()],
                    ambiguous: false,
                    context_terms: Vec::new(),
                },
                AgentAliasConfig {
                    canonical_name: "MCP".to_string(),
                    category: "mcp_or_connector".to_string(),
                    aliases: vec![
                        "MCP".to_string(),
                        "MCP server".to_string(),
                        "Model Context Protocol".to_string(),
                    ],
                    ambiguous: true,
                    context_terms: vec![
                        "server".to_string(),
                        "claude".to_string(),
                        "agent".to_string(),
                        "protocol".to_string(),
                    ],
                },
                AgentAliasConfig {
                    canonical_name: "Cursor".to_string(),
                    category: "coding_assistant".to_string(),
                    aliases: vec![
                        "Cursor".to_string(),
                        "Cursor AI".to_string(),
                        "Cursor IDE".to_string(),
                    ],
                    ambiguous: true,
                    context_terms: vec![
                        "ai".to_string(),
                        "coding".to_string(),
                        "code".to_string(),
                        "ide".to_string(),
                    ],
                },
            ],
        }
    }

    fn mention_names(text: &str) -> Vec<String> {
        detect_mentions_in_text("post-1", text, &test_config(), &HashMap::new())
            .into_iter()
            .map(|mention| mention.agent_name)
            .collect()
    }

    #[test]
    fn normalizes_text_for_alias_matching() {
        assert_eq!(
            normalize_text("Claude-Code, MCP server!"),
            "claude code mcp server"
        );
    }

    #[test]
    fn detects_cavemen_as_caveman_skill() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "cavemen mode is faster for coding agents",
            &test_config(),
            &HashMap::new(),
        );

        assert_eq!(mentions[0].agent_name, "Caveman");
        assert_eq!(mentions[0].category, "skill_or_mode");
    }

    #[test]
    fn detects_ponytail_skill() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "Ponytail helps avoid overengineering",
            &test_config(),
            &HashMap::new(),
        );

        assert_eq!(mentions[0].agent_name, "Ponytail");
        assert_eq!(mentions[0].category, "skill_or_mode");
    }

    #[test]
    fn detects_ponytail_domain_skill() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "Ponytail.dev is useful for Claude Code workflow",
            &test_config(),
            &HashMap::new(),
        );

        let ponytail = mentions
            .iter()
            .find(|mention| mention.agent_name == "Ponytail")
            .expect("Ponytail should be detected from its domain alias");
        assert_eq!(ponytail.category, "skill_or_mode");
        assert!(!ponytail.needs_review);
    }

    #[test]
    fn detects_astryx_known_agent() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "I tried Astryx for agentic workflow",
            &test_config(),
            &HashMap::new(),
        );

        assert!(mentions.iter().any(|mention| mention.agent_name == "Astryx"
            && mention.detection_source == "known_alias"
            && !mention.needs_review));
    }

    #[test]
    fn detects_unknown_candidate_in_agent_context() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "NovaForge is showing up in AI agent discovery threads.",
            &test_config(),
            &HashMap::new(),
        );

        assert!(mentions
            .iter()
            .any(|mention| mention.agent_name == "NovaForge"
                && mention.category == "unknown_candidate"
                && mention.detection_source == "candidate_pattern"
                && mention.needs_review));
    }

    #[test]
    fn accepts_strict_brand_like_unknown_candidates() {
        for (text, expected_name) in [
            (
                "Graphify helps reduce token usage for agent memory.",
                "Graphify",
            ),
            (
                "Headroom is useful for managing agent workflow.",
                "Headroom",
            ),
            ("MemoryFlow improves developer automation.", "MemoryFlow"),
        ] {
            let mentions =
                detect_mentions_in_text("post-brand", text, &test_config(), &HashMap::new());
            assert!(mentions.iter().any(|mention| {
                mention.agent_name == expected_name
                    && mention.category == "unknown_candidate"
                    && mention.review_status == "pending"
                    && mention.needs_review
            }));
        }
    }

    #[test]
    fn entity_gate_rejects_generic_concepts_and_accepts_named_entities() {
        let detector = NamedEntityGateDetector {
            config: test_config(),
        };

        for text in [
            "I built a team of 5 AI agents that run my ENTIRE business. Here's what they do. (save for later)",
            "HTML blocks now work with any agent via MCP.",
            "We built an MCP server and weekly usage tripled.",
            "Who wants to learn Agentic AI?",
        ] {
            assert!(
                detector.detect(text).is_empty(),
                "generic text passed entity gate: {text}"
            );
        }

        let graphify = detector.detect("Graphify helps reduce token usage for agent memory.");
        assert!(graphify.iter().any(|entity| {
            entity.entity_name == "Graphify"
                && entity.category == "unknown_candidate"
                && entity.detection_source == "candidate_pattern"
        }));

        let headroom = detector.detect("Headroom is useful for managing agent workflow.");
        assert!(headroom.iter().any(|entity| {
            entity.entity_name == "Headroom" && entity.category == "unknown_candidate"
        }));

        let ponytail = detector.detect("Ponytail feels useful for Claude Code workflow.");
        assert!(ponytail.iter().any(|entity| {
            entity.entity_name == "Ponytail"
                && entity.category == "skill_or_mode"
                && entity.detection_source == "known_alias"
        }));
        assert!(ponytail.iter().any(|entity| {
            entity.entity_name == "Claude Code"
                && entity.category == "coding_agent"
                && entity.detection_source == "known_alias"
        }));
    }

    #[test]
    fn excludes_common_capitalized_words_from_unknown_candidates() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "And The Any But Here Good I'm APIs Save This are common words in AI agent chatter.",
            &test_config(),
            &HashMap::new(),
        );

        assert!(!mentions.iter().any(|mention| {
            mention.category == "unknown_candidate"
                && matches!(
                    mention.agent_name.as_str(),
                    "And"
                        | "The"
                        | "Any"
                        | "But"
                        | "Here"
                        | "Good"
                        | "I'm"
                        | "APIs"
                        | "Save"
                        | "This"
                )
        }));
    }

    #[test]
    fn excludes_generic_concepts_and_threadbait_fragments_from_candidates() {
        let mentions = detect_mentions_in_text(
            "post-generic-candidates",
            "HTML LLM LLMs MCP API APIs ENTIRE SAME BREATH BREAKING Claude Agentic Agent Come Good Save The And But Here How Any Can Did Don't For I'm It's Everyone GenX'ers Large Action Models Large Language Models appear near AI agent workflow.",
            &test_config(),
            &HashMap::new(),
        );

        assert!(!mentions
            .iter()
            .any(|mention| mention.category == "unknown_candidate"));
    }

    #[test]
    fn keeps_known_aliases_while_tightening_candidates() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "Claude Code and MCP server both help developer workflow.",
            &test_config(),
            &HashMap::new(),
        );

        assert!(mentions
            .iter()
            .any(|mention| mention.agent_name == "Claude Code"
                && mention.detection_source == "known_alias"));
        assert!(mentions.iter().any(
            |mention| mention.agent_name == "MCP" && mention.detection_source == "known_alias"
        ));
    }

    #[test]
    fn suppresses_unknown_fragments_inside_known_aliases() {
        for (text, expected_entity, rejected_fragments) in [
            (
                "Claude Code feels completely different after this. Anthropic quietly released an official plugin that scans your project and recommends Skills, MCP servers, Subagents, and Hooks.",
                "Claude Code",
                vec!["Claude", "Code", "Anthropic"],
            ),
            (
                "Codex CLI is useful for agentic coding workflows.",
                "Codex CLI",
                vec!["Codex", "CLI"],
            ),
            (
                "GitHub Copilot agent mode helped automate code review.",
                "GitHub Copilot",
                vec!["GitHub", "Copilot"],
            ),
        ] {
            let mentions =
                detect_mentions_in_text("post-fragment", text, &test_config(), &HashMap::new());
            assert!(mentions.iter().any(|mention| {
                mention.agent_name == expected_entity && mention.detection_source == "known_alias"
            }));
            assert!(mentions.iter().all(|mention| {
                mention.category != "unknown_candidate"
                    || !rejected_fragments.contains(&mention.agent_name.as_str())
            }));
        }
    }

    #[test]
    fn excludes_platform_and_recruitment_terms_from_unknown_candidates() {
        for candidate in [
            "GenAI",
            "TikTok",
            "Threads",
            "Instagram",
            "Threads TikTok Instagram",
            "Setter/Closer",
            "Appointment Setter",
            "Closer",
            "Appointment Setter/Closer",
            "Agent Engineer",
            "AI Agent Engineer",
            "Copilots",
            "YouTube",
        ] {
            let text = format!("{candidate} appears near an AI agent workflow discussion.");
            let mentions =
                detect_mentions_in_text("post-noise", &text, &test_config(), &HashMap::new());

            assert!(
                mentions
                    .iter()
                    .all(|mention| mention.category != "unknown_candidate"),
                "{candidate} must not enter candidate review"
            );
        }
    }

    #[test]
    fn keeps_known_copilot_alias_while_rejecting_generic_copilots() {
        let mentions = detect_mentions_in_text(
            "post-copilot",
            "GitHub Copilot is useful, but generic Copilots are not a concrete tool name.",
            &test_config(),
            &HashMap::new(),
        );

        assert!(mentions.iter().any(|mention| {
            mention.agent_name == "GitHub Copilot" && mention.detection_source == "known_alias"
        }));
        assert!(mentions.iter().all(|mention| {
            mention.category != "unknown_candidate" || mention.agent_name != "Copilots"
        }));
    }

    #[test]
    fn requires_product_identity_evidence_for_domain_candidates() {
        let launch_mentions = detect_mentions_in_text(
            "post-domain-launch",
            "Introducing the most powerful personal AI agent. folk.com",
            &test_config(),
            &HashMap::new(),
        );
        assert!(launch_mentions.iter().any(|mention| {
            mention.agent_name == "folk.com"
                && mention.category == "unknown_candidate"
                && mention.review_status == "pending"
        }));

        let random_mentions = detect_mentions_in_text(
            "post-domain-random",
            "Read the AI agent article at folk.com for background information.",
            &test_config(),
            &HashMap::new(),
        );
        assert!(random_mentions
            .iter()
            .all(|mention| mention.agent_name != "folk.com"));
    }

    #[test]
    fn known_alias_suppresses_weak_candidates_but_keeps_strong_candidates() {
        let weak_mentions = detect_mentions_in_text(
            "post-known-with-noise",
            "Claude Code is more Agentic than generative and there is not a single app from any major or minor company that isn't using some form of agentic AI in 2026. She mentions TikTok and Instagram.",
            &test_config(),
            &HashMap::new(),
        );
        assert!(weak_mentions
            .iter()
            .any(|mention| mention.agent_name == "Claude Code"));
        assert!(weak_mentions
            .iter()
            .all(|mention| mention.category != "unknown_candidate"));

        let strong_mentions = detect_mentions_in_text(
            "post-known-with-strong-candidate",
            "Claude Code works with Graphify for agent memory.",
            &test_config(),
            &HashMap::new(),
        );
        assert!(strong_mentions.iter().any(|mention| {
            mention.agent_name == "Graphify" && mention.category == "unknown_candidate"
        }));
    }

    #[test]
    fn excludes_large_model_concepts_from_candidates_and_entity_gate() {
        let text = "Agentic AI is powered by both Large Language Models LLMs and Large Action Models LAMs.";
        let mentions =
            detect_mentions_in_text("post-model-concepts", text, &test_config(), &HashMap::new());
        let detector = NamedEntityGateDetector {
            config: test_config(),
        };

        assert!(!mentions
            .iter()
            .any(|mention| mention.category == "unknown_candidate"));
        assert!(detector.detect(text).is_empty());
    }

    #[test]
    fn detects_explainx_registry() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "ExplainX has many AI agent skills",
            &test_config(),
            &HashMap::new(),
        );

        assert_eq!(mentions[0].agent_name, "ExplainX");
        assert_eq!(mentions[0].category, "registry_or_discovery");
    }

    #[test]
    fn detects_explainx_domain_registry() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "explainx.ai has a useful registry",
            &test_config(),
            &HashMap::new(),
        );

        assert_eq!(mentions[0].agent_name, "ExplainX");
        assert_eq!(mentions[0].category, "registry_or_discovery");
    }

    #[test]
    fn detects_mcp_when_context_is_connector_related() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "MCP server for Claude Code",
            &test_config(),
            &HashMap::new(),
        );

        let mcp = mentions
            .iter()
            .find(|mention| mention.agent_name == "MCP")
            .expect("MCP should be detected with connector context");
        assert_eq!(mcp.category, "mcp_or_connector");
    }

    #[test]
    fn detects_model_context_protocol_as_mcp() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "Model Context Protocol is useful",
            &test_config(),
            &HashMap::new(),
        );

        assert_eq!(mentions[0].agent_name, "MCP");
        assert_eq!(mentions[0].category, "mcp_or_connector");
    }

    #[test]
    fn does_not_detect_mcp_without_context() {
        assert!(!mention_names("random MCP note").contains(&"MCP".to_string()));
    }

    #[test]
    fn does_not_detect_cursor_without_ai_context() {
        assert!(!mention_names("my cursor is broken").contains(&"Cursor".to_string()));
    }

    #[test]
    fn detects_cursor_with_ai_context() {
        assert!(mention_names("Cursor AI is useful for coding").contains(&"Cursor".to_string()));
    }
}
