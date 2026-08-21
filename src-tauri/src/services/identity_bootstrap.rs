use std::collections::{BTreeMap, HashSet};

use crate::models::multi_source::{
    normalize_entity_name, AliasBootstrapResult, AliasProvenance, AliasSourceScope,
    NewCanonicalEntity, NewEntityAlias, PrimaryEntityType,
};
use crate::services::entity_detector;
use crate::services::multi_source_repository::MultiSourceRepository;

pub fn bootstrap_curated_aliases(
    repository: &MultiSourceRepository,
) -> Result<AliasBootstrapResult, String> {
    let config = entity_detector::load_aliases_config()?;
    let mut result = AliasBootstrapResult {
        configured_entities: config.agents.len(),
        entities_created: 0,
        entities_reused: 0,
        entity_conflicts: 0,
        aliases_created: 0,
        aliases_existing: 0,
        ambiguous_aliases: 0,
        skipped_entities: 0,
        type_mapping_counts: BTreeMap::new(),
        skipped_reasons: Vec::new(),
    };

    for configured in config.agents {
        let Some(primary_type) = map_primary_type(&configured.category, &configured.canonical_name)
        else {
            result.skipped_entities += 1;
            result.skipped_reasons.push(format!(
                "{}: unsupported category {}",
                configured.canonical_name, configured.category
            ));
            continue;
        };
        *result
            .type_mapping_counts
            .entry(primary_type.as_str().to_string())
            .or_default() += 1;

        let matches =
            repository.lookup_canonical_entities_by_normalized_name(&configured.canonical_name)?;
        let entity = match matches.as_slice() {
            [] => {
                result.entities_created += 1;
                repository.create_canonical_entity(&NewCanonicalEntity {
                    canonical_name: configured.canonical_name.clone(),
                    primary_type,
                    description: None,
                    primary_website: None,
                    primary_repository: None,
                })?
            }
            [entity] => {
                result.entities_reused += 1;
                entity.clone()
            }
            _ => {
                result.entity_conflicts += 1;
                result.skipped_entities += 1;
                result.skipped_reasons.push(format!(
                    "{}: multiple canonical entities share normalized name",
                    configured.canonical_name
                ));
                continue;
            }
        };

        let context_terms_json = if configured.context_terms.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&configured.context_terms)
                    .map_err(|error| format!("Alias context serialization failed: {error}"))?,
            )
        };
        let mut seen = HashSet::new();
        for alias in std::iter::once(configured.canonical_name.as_str())
            .chain(configured.aliases.iter().map(String::as_str))
        {
            let normalized = normalize_entity_name(alias);
            if normalized.is_empty() || !seen.insert(normalized) {
                continue;
            }
            let created = repository.create_entity_alias(&NewEntityAlias {
                entity_id: entity.entity_id.clone(),
                alias: alias.to_string(),
                source_scope: AliasSourceScope::Global,
                provenance: AliasProvenance::BootstrapYaml,
                is_ambiguous: configured.ambiguous,
                context_terms_json: context_terms_json.clone(),
            })?;
            if created.inserted {
                result.aliases_created += 1;
            } else {
                result.aliases_existing += 1;
            }
            if configured.ambiguous {
                result.ambiguous_aliases += 1;
            }
        }
    }

    Ok(result)
}

fn map_primary_type(category: &str, canonical_name: &str) -> Option<PrimaryEntityType> {
    match category {
        "coding_agent" | "coding_assistant" => Some(PrimaryEntityType::AgentTool),
        "generic_agent_framework" => Some(PrimaryEntityType::FrameworkSdk),
        "skill_or_mode" => Some(PrimaryEntityType::SkillMode),
        "registry_or_discovery" => Some(PrimaryEntityType::RegistryDiscovery),
        "app_builder" => Some(PrimaryEntityType::AppBuilder),
        "mcp_or_connector" if normalize_entity_name(canonical_name) == "mcp" => {
            Some(PrimaryEntityType::Protocol)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn real_yaml_bootstrap_is_idempotent() {
        let path = test_database_path("real-yaml-bootstrap");
        cleanup_database_files(&path);
        let repository =
            MultiSourceRepository::open_at(&path).expect("test repository should open");

        let first = bootstrap_curated_aliases(&repository).expect("first bootstrap should succeed");
        let first_aliases = repository
            .list_aliases_by_source_scope(AliasSourceScope::Global)
            .expect("first aliases should load");
        let second =
            bootstrap_curated_aliases(&repository).expect("second bootstrap should succeed");
        let second_aliases = repository
            .list_aliases_by_source_scope(AliasSourceScope::Global)
            .expect("second aliases should load");

        assert_eq!(first.configured_entities, 26);
        assert_eq!(first.entities_created, 26);
        assert_eq!(first.entities_reused, 0);
        assert_eq!(first.entity_conflicts, 0);
        assert_eq!(first.skipped_entities, 0);
        assert_eq!(first.aliases_created, first_aliases.len());
        assert!(first.aliases_created > first.entities_created);
        assert_eq!(second.entities_created, 0);
        assert_eq!(second.entities_reused, 26);
        assert_eq!(second.aliases_created, 0);
        assert_eq!(second.aliases_existing, first.aliases_created);
        assert_eq!(first_aliases.len(), second_aliases.len());
        assert_eq!(first.type_mapping_counts, second.type_mapping_counts);
        println!(
            "bootstrap entities={} aliases={} ambiguous={} mappings={:?} skipped={}",
            first.entities_created,
            first.aliases_created,
            first.ambiguous_aliases,
            first.type_mapping_counts,
            first.skipped_entities
        );

        drop(repository);
        cleanup_database_files(&path);
    }

    fn test_database_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ai-agent-trend-radar-{name}.duckdb"))
    }

    fn cleanup_database_files(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(path.with_extension("duckdb.wal"));
        let _ = fs::remove_file(path.with_extension("duckdb.tmp"));
    }
}
