use std::collections::HashSet;

use crate::models::entities::{
    AgentMentionForIdentityLinkage, IdentityResolutionStatus, MentionIdentityLinkagePreview,
    MentionIdentityLinkageResult, MentionIdentityResolution,
};
use crate::models::multi_source::{AliasEntityMatch, AliasSourceScope, PrimaryEntityType};
use crate::services::duckdb_service;
use crate::services::identity_bootstrap;
use crate::services::multi_source_repository::MultiSourceRepository;

pub fn link_agent_mentions_to_entities() -> Result<MentionIdentityLinkageResult, String> {
    let mentions = duckdb_service::load_agent_mentions_for_identity_linkage()?;
    let repository = MultiSourceRepository::open()?;
    identity_bootstrap::bootstrap_curated_aliases(&repository)?;

    let mut resolutions = Vec::new();
    let mut preview = Vec::new();
    let mut resolved_count = 0;
    let mut missing_alias_count = 0;
    let mut ambiguous_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    for mention in &mentions {
        match resolve_mention_identity(&repository, mention, AliasSourceScope::Threads) {
            Ok(resolution) => {
                match resolution.status {
                    IdentityResolutionStatus::Resolved => resolved_count += 1,
                    IdentityResolutionStatus::MissingAlias => missing_alias_count += 1,
                    IdentityResolutionStatus::Ambiguous => ambiguous_count += 1,
                    IdentityResolutionStatus::Skipped => skipped_count += 1,
                    IdentityResolutionStatus::Unresolved => error_count += 1,
                }
                if preview.len() < 12 {
                    preview.push(preview_from_resolution(&resolution));
                }
                resolutions.push(resolution);
            }
            Err(error) => {
                error_count += 1;
                let resolution =
                    base_resolution(mention, IdentityResolutionStatus::Unresolved, error, 0.0);
                if preview.len() < 12 {
                    preview.push(preview_from_resolution(&resolution));
                }
                resolutions.push(resolution);
            }
        }
    }

    drop(repository);
    duckdb_service::save_mention_identity_resolutions(&resolutions)?;
    Ok(MentionIdentityLinkageResult {
        resolved_count,
        missing_alias_count,
        ambiguous_count,
        skipped_count,
        error_count,
        message: format!(
            "Identity linkage completed for {} mentions: {resolved_count} resolved, {missing_alias_count} missing alias, {ambiguous_count} ambiguous, {skipped_count} skipped, {error_count} errors.",
            mentions.len()
        ),
        preview,
    })
}

pub fn resolve_mention_identity(
    repository: &MultiSourceRepository,
    mention: &AgentMentionForIdentityLinkage,
    source_scope: AliasSourceScope,
) -> Result<MentionIdentityResolution, String> {
    let matches =
        repository.lookup_entities_by_normalized_alias(&mention.agent_name, source_scope)?;
    let scoped_matches = preferred_scope_matches(matches, source_scope);

    if scoped_matches.is_empty() {
        let status = if repository.has_inactive_alias(&mention.agent_name, source_scope)? {
            IdentityResolutionStatus::Skipped
        } else {
            IdentityResolutionStatus::MissingAlias
        };
        let reason = if status == IdentityResolutionStatus::Skipped {
            "Matching alias exists but is archived or belongs to an archived entity.".to_string()
        } else {
            "No active alias matches this mention in the source or global scope.".to_string()
        };
        return Ok(base_resolution(mention, status, reason, 0.0));
    }

    let entity_ids = scoped_matches
        .iter()
        .map(|candidate| candidate.entity.entity_id.as_str())
        .collect::<HashSet<_>>();
    if entity_ids.len() > 1 {
        return Ok(base_resolution(
            mention,
            IdentityResolutionStatus::Ambiguous,
            format!(
                "Alias matches {} active canonical entities in the preferred scope.",
                entity_ids.len()
            ),
            0.0,
        ));
    }

    let candidate = &scoped_matches[0];
    if !category_is_compatible(&mention.category, candidate.entity.primary_type) {
        return Ok(base_resolution(
            mention,
            IdentityResolutionStatus::Skipped,
            format!(
                "Alias category {} is incompatible with canonical type {}.",
                mention.category,
                candidate.entity.primary_type.as_str()
            ),
            0.0,
        ));
    }

    if candidate.alias.is_ambiguous
        && !ambiguous_alias_context_matches(candidate, &mention.source_snippet)?
    {
        return Ok(base_resolution(
            mention,
            IdentityResolutionStatus::Ambiguous,
            "Alias is marked ambiguous and the mention context does not strongly match its configured context terms."
                .to_string(),
            0.0,
        ));
    }

    let confidence = if candidate.alias.is_ambiguous {
        0.9
    } else {
        1.0
    };
    Ok(MentionIdentityResolution {
        mention_id: mention.mention_id.clone(),
        agent_name: mention.agent_name.clone(),
        entity_id: Some(candidate.entity.entity_id.clone()),
        canonical_entity_name: Some(candidate.entity.canonical_name.clone()),
        status: IdentityResolutionStatus::Resolved,
        reason: format!(
            "Resolved through active {} alias '{}'{}.",
            candidate.alias.source_scope.as_str(),
            candidate.alias.alias,
            if candidate.alias.is_ambiguous {
                " with matching context"
            } else {
                ""
            }
        ),
        confidence,
    })
}

fn preferred_scope_matches(
    matches: Vec<AliasEntityMatch>,
    source_scope: AliasSourceScope,
) -> Vec<AliasEntityMatch> {
    let exact = matches
        .iter()
        .filter(|candidate| candidate.alias.source_scope == source_scope)
        .cloned()
        .collect::<Vec<_>>();
    if exact.is_empty() {
        matches
            .into_iter()
            .filter(|candidate| candidate.alias.source_scope == AliasSourceScope::Global)
            .collect()
    } else {
        exact
    }
}

fn ambiguous_alias_context_matches(
    candidate: &AliasEntityMatch,
    context: &str,
) -> Result<bool, String> {
    let Some(context_terms_json) = candidate.alias.context_terms_json.as_deref() else {
        return Ok(false);
    };
    let context_terms: Vec<String> = serde_json::from_str(context_terms_json).map_err(|error| {
        format!(
            "Alias context terms are invalid for {}: {error}",
            candidate.entity.canonical_name
        )
    })?;
    let normalized_context = normalize_context(context);
    Ok(context_terms
        .iter()
        .any(|term| contains_normalized_phrase(&normalized_context, term)))
}

fn normalize_context(value: &str) -> String {
    let normalized = value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>();
    format!(
        " {} ",
        normalized.split_whitespace().collect::<Vec<_>>().join(" ")
    )
}

fn contains_normalized_phrase(normalized_context: &str, phrase: &str) -> bool {
    let normalized_phrase = normalize_context(phrase);
    normalized_context.contains(&normalized_phrase)
}

fn category_is_compatible(category: &str, primary_type: PrimaryEntityType) -> bool {
    match category {
        "coding_agent" | "coding_assistant" => primary_type == PrimaryEntityType::AgentTool,
        "generic_agent_framework" => primary_type == PrimaryEntityType::FrameworkSdk,
        "skill_or_mode" => primary_type == PrimaryEntityType::SkillMode,
        "mcp_or_connector" => matches!(
            primary_type,
            PrimaryEntityType::Protocol | PrimaryEntityType::ConnectorPlugin
        ),
        "registry_or_discovery" => primary_type == PrimaryEntityType::RegistryDiscovery,
        "app_builder" => primary_type == PrimaryEntityType::AppBuilder,
        "unknown_candidate" | "unknown" | "" => true,
        _ => false,
    }
}

fn base_resolution(
    mention: &AgentMentionForIdentityLinkage,
    status: IdentityResolutionStatus,
    reason: String,
    confidence: f64,
) -> MentionIdentityResolution {
    MentionIdentityResolution {
        mention_id: mention.mention_id.clone(),
        agent_name: mention.agent_name.clone(),
        entity_id: None,
        canonical_entity_name: None,
        status,
        reason,
        confidence,
    }
}

fn preview_from_resolution(
    resolution: &MentionIdentityResolution,
) -> MentionIdentityLinkagePreview {
    MentionIdentityLinkagePreview {
        mention_name: resolution.agent_name.clone(),
        resolution_status: resolution.status.as_str().to_string(),
        canonical_entity_name: resolution.canonical_entity_name.clone().unwrap_or_default(),
        reason: resolution.reason.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::models::multi_source::{AliasProvenance, NewCanonicalEntity, NewEntityAlias};

    use super::*;

    #[test]
    fn resolves_known_alias_and_variant() {
        with_repository("known-alias", |repository| {
            let entity = create_entity_with_aliases(
                repository,
                "Claude Code",
                PrimaryEntityType::AgentTool,
                &["Claude Code", "claude-code"],
                AliasSourceScope::Global,
                false,
                None,
            );

            for name in ["Claude Code", "claude-code"] {
                let resolution = resolve_mention_identity(
                    repository,
                    &mention(name, "coding_agent", "Claude Code helps with coding."),
                    AliasSourceScope::Threads,
                )
                .expect("known alias should resolve");
                assert_eq!(resolution.status, IdentityResolutionStatus::Resolved);
                assert_eq!(
                    resolution.entity_id.as_deref(),
                    Some(entity.entity_id.as_str())
                );
                assert_eq!(
                    resolution.canonical_entity_name.as_deref(),
                    Some("Claude Code")
                );
            }
        });
    }

    #[test]
    fn source_scoped_alias_is_preferred_over_global_alias() {
        with_repository("source-preference", |repository| {
            create_entity_with_aliases(
                repository,
                "Cursor Product",
                PrimaryEntityType::AgentTool,
                &["Cursor"],
                AliasSourceScope::Global,
                false,
                None,
            );
            let explainx = create_entity_with_aliases(
                repository,
                "Cursor Resource",
                PrimaryEntityType::AgentTool,
                &["Cursor"],
                AliasSourceScope::ExplainX,
                false,
                None,
            );

            let resolution = resolve_mention_identity(
                repository,
                &mention("Cursor", "coding_agent", "Cursor resource entry"),
                AliasSourceScope::ExplainX,
            )
            .expect("source-scoped alias should resolve");
            assert_eq!(resolution.status, IdentityResolutionStatus::Resolved);
            assert_eq!(
                resolution.entity_id.as_deref(),
                Some(explainx.entity_id.as_str())
            );
        });
    }

    #[test]
    fn ambiguous_and_missing_aliases_remain_unlinked() {
        with_repository("ambiguous-missing", |repository| {
            create_entity_with_aliases(
                repository,
                "Codex CLI",
                PrimaryEntityType::AgentTool,
                &["Codex"],
                AliasSourceScope::Global,
                true,
                Some(r#"["openai","coding","cli"]"#),
            );

            let ambiguous = resolve_mention_identity(
                repository,
                &mention("Codex", "coding_agent", "Codex"),
                AliasSourceScope::Threads,
            )
            .expect("ambiguous alias should return a decision");
            assert_eq!(ambiguous.status, IdentityResolutionStatus::Ambiguous);
            assert!(ambiguous.entity_id.is_none());
            assert!(ambiguous.reason.contains("ambiguous"));

            let missing = resolve_mention_identity(
                repository,
                &mention(
                    "UnknownNewTool",
                    "unknown_candidate",
                    "UnknownNewTool launched.",
                ),
                AliasSourceScope::Threads,
            )
            .expect("missing alias should return a decision");
            assert_eq!(missing.status, IdentityResolutionStatus::MissingAlias);
            assert!(missing.entity_id.is_none());
        });
    }

    fn mention(name: &str, category: &str, source_snippet: &str) -> AgentMentionForIdentityLinkage {
        AgentMentionForIdentityLinkage {
            mention_id: format!("mention-{name}"),
            agent_name: name.to_string(),
            category: category.to_string(),
            source_snippet: source_snippet.to_string(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn create_entity_with_aliases(
        repository: &MultiSourceRepository,
        canonical_name: &str,
        primary_type: PrimaryEntityType,
        aliases: &[&str],
        source_scope: AliasSourceScope,
        is_ambiguous: bool,
        context_terms_json: Option<&str>,
    ) -> crate::models::multi_source::CanonicalEntity {
        let entity = repository
            .create_canonical_entity(&NewCanonicalEntity {
                canonical_name: canonical_name.to_string(),
                primary_type,
                description: None,
                primary_website: None,
                primary_repository: None,
            })
            .expect("canonical entity should insert");
        for alias in aliases {
            repository
                .create_entity_alias(&NewEntityAlias {
                    entity_id: entity.entity_id.clone(),
                    alias: (*alias).to_string(),
                    source_scope,
                    provenance: AliasProvenance::Manual,
                    is_ambiguous,
                    context_terms_json: context_terms_json.map(ToString::to_string),
                })
                .expect("entity alias should insert");
        }
        entity
    }

    fn with_repository<F>(name: &str, test: F)
    where
        F: FnOnce(&MultiSourceRepository),
    {
        let path = test_database_path(name);
        cleanup_database_files(&path);
        let repository =
            MultiSourceRepository::open_at(&path).expect("test repository should open");
        test(&repository);
        drop(repository);
        cleanup_database_files(&path);
    }

    fn test_database_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ai-agent-trend-radar-identity-linker-{name}.duckdb"
        ))
    }

    fn cleanup_database_files(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(path.with_extension("duckdb.wal"));
        let _ = fs::remove_file(path.with_extension("duckdb.tmp"));
    }
}
