use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::models::entities::{
    AgentAliasConfig, AliasesConfig, DetectedAgentMention, EntityDetectionResult,
};
use crate::services::duckdb_service;

const ALIASES_CONFIG_PATH: &str = "config/aliases.yml";
const PREVIEW_LIMIT: usize = 12;
const SNIPPET_LIMIT: usize = 180;

pub fn detect_agent_mentions() -> Result<EntityDetectionResult, String> {
    let config = load_aliases_config()?;
    if config.agents.is_empty() {
        return Err("No agent aliases configured in config/aliases.yml".to_string());
    }

    let posts = duckdb_service::load_raw_posts_for_detection()?;
    let mut mentions = Vec::new();

    for post in &posts {
        mentions.extend(detect_mentions_in_text(&post.post_id, &post.text, &config));
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
) -> Vec<DetectedAgentMention> {
    let normalized_text = normalize_text(text);
    if normalized_text.is_empty() {
        return Vec::new();
    }

    let mut seen_agents = HashSet::new();
    let mut mentions = Vec::new();
    let known_aliases = known_aliases(config);

    for agent in &config.agents {
        if seen_agents.contains(&agent.canonical_name) {
            continue;
        }

        if let Some((matched_alias, confidence)) = detect_agent_alias(agent, &normalized_text) {
            seen_agents.insert(agent.canonical_name.clone());
            mentions.push(DetectedAgentMention {
                mention_id: stable_mention_id(post_id, &agent.canonical_name),
                post_id: post_id.to_string(),
                agent_name: agent.canonical_name.clone(),
                agent_alias: matched_alias.clone(),
                category: agent.category.clone(),
                detection_source: "known_alias".to_string(),
                needs_review: false,
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
            || known_aliases.contains(&normalized_candidate)
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
        candidates.push(DetectedAgentMention {
            mention_id: stable_mention_id(post_id, &format!("candidate::{candidate}")),
            post_id: post_id.to_string(),
            agent_name: candidate.clone(),
            agent_alias: candidate,
            category: "unknown_candidate".to_string(),
            detection_source: "candidate_pattern".to_string(),
            needs_review: true,
            region: "unknown".to_string(),
            confidence: 0.62,
            match_confidence: 0.62,
            relevance_score: 0.66,
            sentiment: "unknown".to_string(),
            cost_signal: "none".to_string(),
            source_snippet: source_snippet(text),
        });
    }

    candidates
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

fn has_candidate_context(normalized_text: &str) -> bool {
    candidate_context_terms()
        .iter()
        .any(|term| contains_alias(normalized_text, term))
}

fn candidate_context_terms() -> [&'static str; 12] {
    [
        "ai",
        "agent",
        "agents",
        "agentic",
        "coding",
        "code",
        "tools",
        "developer",
        "workflow",
        "workflows",
        "server",
        "skills",
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

        candidates.push(phrase.join(" "));
        index = next;
    }

    candidates
}

fn clean_candidate_token(token: &str) -> &str {
    token.trim_matches(|character: char| {
        !(character.is_alphanumeric() || character == '.' || character == '-' || character == '_')
    })
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
    let stop_phrases = [
        "ada",
        "agent",
        "ai",
        "ai agent",
        "agent trend radar",
        "ai agent trend",
        "ai coding",
        "developer",
        "model context protocol",
        "testing",
        "threads",
        "tools",
        "tools ai",
        "trend radar",
    ];

    stop_phrases.contains(&normalized_candidate)
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
        detect_mentions_in_text("post-1", text, &test_config())
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
        );

        assert_eq!(mentions[0].agent_name, "Ponytail");
        assert_eq!(mentions[0].category, "skill_or_mode");
        assert!(!mentions[0].needs_review);
    }

    #[test]
    fn detects_astryx_known_agent() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "I tried Astryx for agentic workflow",
            &test_config(),
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
        );

        assert!(mentions
            .iter()
            .any(|mention| mention.agent_name == "NovaForge"
                && mention.category == "unknown_candidate"
                && mention.detection_source == "candidate_pattern"
                && mention.needs_review));
    }

    #[test]
    fn detects_explainx_registry() {
        let mentions = detect_mentions_in_text(
            "post-1",
            "ExplainX has many AI agent skills",
            &test_config(),
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
        );

        assert_eq!(mentions[0].agent_name, "ExplainX");
        assert_eq!(mentions[0].category, "registry_or_discovery");
    }

    #[test]
    fn detects_mcp_when_context_is_connector_related() {
        let mentions =
            detect_mentions_in_text("post-1", "MCP server for Claude Code", &test_config());

        assert_eq!(mentions[0].agent_name, "MCP");
        assert_eq!(mentions[0].category, "mcp_or_connector");
    }

    #[test]
    fn detects_model_context_protocol_as_mcp() {
        let mentions =
            detect_mentions_in_text("post-1", "Model Context Protocol is useful", &test_config());

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
