# DuckDB Schema Foundation

This is the MVP local storage boundary for AI Agent Trend Radar. It is intentionally small and can be migrated later when ingestion and reporting requirements become clearer.

## Storage Boundary

- `threads_posts_raw` stores raw Threads post records and source metadata.
- `crawl_runs` stores summary diagnostics for discovery crawl runs.
- `agent_mentions` stores normalized AI agent/tool mentions detected inside raw posts.
- `entity_review_decisions` stores durable approve/ignore decisions for unknown candidates.
- `weekly_agent_metrics` stores report-ready weekly aggregates by agent and region.
- `canonical_entities` stores opaque, source-independent AI technology identities.
- `source_collection_runs` stores bounded external-source collection attempts.
- `source_records` stores durable external objects outside the Threads post model.
- `source_observations` stores append-oriented historical snapshots of source records.
- `source_record_entity_links` stores reviewed many-to-many relationships between external records and canonical entities.

No Threads access token, API key, or app secret should ever be stored in DuckDB.

## Tables

### threads_posts_raw

Raw local archive of Threads posts collected for trend analysis.

- `post_id`: Threads post identifier.
- `thread_id`: Optional parent/thread identifier.
- `author_id`: Threads author identifier from API data.
- `author_username`: Display username if available.
- `author_display_name`: Optional display name from fallback sources when available.
- `text`: Post text.
- `text_missing`: `true` when keyword/detail response did not provide text or caption.
- `permalink`: Optional post URL.
- `media_type`: Optional Threads media type from the API response.
- `source_type`: Source connector label, such as `apify_threads_scraper` for the experimental fallback.
- `source_seed_keyword`: Seed/search keyword that produced the raw post when available.
- `keyword_match`: Source-provided keyword match diagnostic when available.
- `language`: Optional detected or API-provided language.
- `region_hint`: Optional region hint such as `indonesia`, `global`, or `unknown`.
- `region_confidence`: Rule-based classifier confidence for the post region.
- `region_reason`: Short explainable reason for the post region label.
- Engagement fields: like, reply, repost, quote, share, and view counts.
- `posted_at`: Post timestamp from Threads.
- `collected_at`: Local collection timestamp.
- `raw_json`: Optional raw API payload as text for replay/debugging.

### crawl_runs

Discovery crawler run history for local diagnostics and demo readiness.

- `id`: Local crawl run identifier.
- `mode`: Crawl mode, such as `real_threads`, `sample_mock`, or mock detail validation mode.
- `seed_group`: Seed group requested by the UI, such as `all`, `global`, or `indonesia`.
- `max_per_seed`: Maximum posts accepted per seed.
- `seeds_processed`: Number of configured seeds processed.
- `fetched_total`: Total post records returned by keyword search before cross-seed dedupe.
- `saved_total`: Unique raw posts saved to `threads_posts_raw`.
- `duplicates_skipped`: Duplicate Threads post IDs skipped across seeds.
- `zero_result_seeds`: Number of seeds where keyword search succeeded but returned no posts.
- `failed_seeds`: Number of seeds with permission/API/request errors.
- `detail_fetched_total`: Number of ID-only search results resolved through post detail fetch.
- `detail_failed_total`: Number of post detail fetch failures.
- `text_missing_total`: Number of posts where detail fetch still did not provide text.
- `started_at`: Local run start timestamp as Unix milliseconds text.
- `finished_at`: Local run finish timestamp as Unix milliseconds text.
- `duration_ms`: Run duration in milliseconds.
- `status`: Local summary status such as `completed`, `completed_with_diagnostics`, or `needs_attention`.
- `error_summary`: Safe error summary without tokens or secrets.

Seed-level diagnostics are returned by the `run_discovery_crawl` command response for UI display. They are not persisted yet, keeping the MVP schema focused.

### agent_mentions

Normalized entity extraction results derived from raw posts.

- `mention_id`: Stable local mention identifier.
- `post_id`: Source post identifier.
- `agent_name`: Normalized agent/tool name.
- `agent_alias`: Matched alias or raw mention text.
- `category`: MVP entity category such as `coding_agent`, `skill_or_mode`, `mcp_or_connector`, `registry_or_discovery`, or `unknown_candidate`.
- `detection_source`: `known_alias` for configured aliases or `candidate_pattern` for rule-based discovery candidates.
- `needs_review`: `true` for candidate entities that should be manually reviewed before being treated as known entities.
- `review_status`: Review workflow state: `pending`, `approved`, or `ignored`. Known aliases default to `approved`; new unknown candidates default to `pending`.
- `reviewed_as`: Optional canonical name assigned during candidate approval.
- `reviewed_category`: Optional approved category assigned during candidate approval.
- `review_note`: Optional local reviewer note.
- `reviewed_at`: Local timestamp for the latest review action.
- `region`: `indonesia`, `global`, or `unknown`.
- `region_confidence`: Rule-based classifier confidence copied from the source post classification.
- `region_reason`: Short explainable reason copied from the source post classification.
- `confidence`: Numeric confidence from deterministic rules or future classifier.
- `match_confidence`: Alias/context match confidence from deterministic entity rules.
- `relevance_score`: Lightweight score for whether the mention appears in an agent/tool context.
- `sentiment`: Rule-based MVP sentiment label: `positive`, `neutral`, `negative`, `mixed`, or `unknown`.
- `sentiment_confidence`: Rule-based classifier confidence for the sentiment label.
- `sentiment_reason`: Short explainable reason for the sentiment label.
- `cost_signal`: Rule-based MVP cost label: `not_mentioned`, `cost_positive`, `cost_negative_boros`, or `cost_mixed`.
- `cost_confidence`: Rule-based classifier confidence for the cost label.
- `cost_reason`: Short explainable reason for the cost label.
- `source_snippet`: Short post text snippet for UI preview and local audit.
- `detected_at`: Local detection timestamp.

### entity_review_decisions

Durable candidate review registry used to apply approve/ignore decisions to future detections.

- `id`: Normalized case-insensitive candidate key.
- `candidate_name`: Original candidate display name.
- `normalized_name`: Canonical entity name used when status is `approved`.
- `category`: Approved entity category used when status is `approved`.
- `status`: Durable decision status: `approved` or `ignored`.
- `note`: Optional reviewer note.
- `created_at`: Local creation timestamp.
- `updated_at`: Local update timestamp.

When an unknown candidate is detected, the entity detector checks this registry. Approved candidates are saved as `reviewed_candidate` mentions with `needs_review = false`; ignored candidates are saved with `review_status = ignored` and excluded from weekly metrics.

### weekly_agent_metrics

Aggregated weekly reporting table.

- Primary key: `week_start`, `region`, `agent_name`.
- `week_start`: Start date of the weekly bucket.
- `week_end`: End date of the weekly bucket.
- `region`: `indonesia`, `global`, or `unknown`.
- `agent_name`: Normalized agent/tool name.
- `category`: MVP entity category copied from `agent_mentions`.
- `mentions`: Mention count for the agent/region/week.
- `mention_count`: Compatibility alias for mention count.
- `unique_author_count`: Placeholder for future author-aware metrics; currently `0`.
- Sentiment counts: positive, neutral, negative, and mixed.
- Cost counts: not mentioned, cost positive, cost negative/boros, and cost mixed.
- Percentages: positive %, negative %, and cost negative/boros %.
- `trend_score`: MVP ranking score.
- `computed_at`: Local computation timestamp.

Weekly aggregation includes only `known_alias` mentions and approved `reviewed_candidate` mentions. Pending or ignored candidates and generic concepts such as standalone `MCP`, `AI Agent`, `HTML`, or `LLM` are excluded from Top Indonesia/Global/Unknown metrics so unreviewed or non-concrete discoveries do not pollute rankings.

Rows are grouped by `week_start`, `region`, canonical entity key, and `category`. The canonical key is based on `lower(trim(agent_name))`, so casing or source-record variants such as `Claude Code` and `claude code` collapse into one row per region/week/category.

The MVP dashboard and report export loaders select only the maximum available `week_start`. Historical rows remain stored, while Top Indonesia/Global/Unknown shows one canonical row per region for the latest week.

MVP trend score formula:

```text
mentions * 10
+ positive_count * 3
+ mixed_count * 1
- negative_count * 2
- cost_negative_boros_count * 1
```

The score formula should move to `config/scoring.yml` when the ranking design stabilizes.

## Multi-Source Foundation

The Phase A multi-source tables are additive. They do not replace `threads_posts_raw`, add canonical IDs to `agent_mentions`, or change weekly aggregation.

### canonical_entities

Source-independent identities for tools, frameworks, skills, protocols, connectors, registries, and app builders.

- Primary key: opaque DuckDB `UUID` in `entity_id`.
- `canonical_name`: current display name.
- `normalized_name`: lookup aid only; it is intentionally not unique.
- `primary_type`: controlled type such as `agent_tool`, `framework_sdk`, or `skill_mode`.
- `status`: `active` or `archived`.
- Optional descriptive metadata: description, primary website, and primary repository.

Two distinct entities may have the same normalized display name. Identity must never be inferred from this field alone.

### source_collection_runs

One bounded attempt to observe an external source.

- Primary key: `collection_run_id` UUID.
- Controlled source values are validated in Rust rather than by a database constraint.
- Modes: `scheduled`, `manual`, `import`, or `replay`.
- Statuses: `running`, `completed`, `partial`, or `failed`.
- `scope_json` describes the requested surface/window without coupling schema to one source.
- `records_seen` and `observations_saved` update transactionally with successful observation inserts.

A missing observation means only “not observed.” A failed or partial run must not imply confirmed absence.

### source_records

A durable object on an external ecosystem source, such as an ExplainX profile or GitHub repository.

- Primary key: `source_record_id` UUID.
- Unique external identity: `(source, source_record_key)`.
- `resolution_state`: `single_entity`, `multiple_entities`, `no_product_entity`, or `unresolved`.
- Mutable descriptive metadata is updated during upsert while `first_seen_at` remains stable and `last_seen_at` advances.
- Source-specific metadata remains optional JSON text.

Source records remain separate from canonical entities. ExplainX or future ecosystem records must not be inserted into `threads_posts_raw`.

### source_observations

Append-oriented snapshots of a source record during a collection run.

- Primary key: `observation_id` UUID.
- Foreign keys reference `source_collection_runs` and `source_records`.
- Common nullable metrics include rank, source score, views, installs, GitHub stars, and upvotes.
- `source_payload_json` preserves the complete source-specific observation payload.
- Same-run duplicate protection uses `(collection_run_id, source_record_id, surface, observation_kind, time_window)`.

The same record and payload may appear in different runs; those rows are valid historical observations and are not deduplicated across runs.

### source_record_entity_links

Reviewed many-to-many relationships between external records and canonical entities.

- Primary key: `link_id` UUID.
- Unique pair: `(source_record_id, entity_id)`.
- Relationship types: `same_entity`, `child_resource`, `related_entity`, or `mentioned_entity`.
- Review states: `pending`, `approved`, `rejected`, or `ambiguous`.
- Match confidence is diagnostic only and never causes automatic approval.
- Optional `evidence_json` preserves resolution evidence.

Service-level validation allows no-product and unresolved records while preventing an approved link from being attached to a record currently classified as `no_product_entity`.

## Entity Identity Persistence

The IMP-02 identity tables are additive and are not connected to Tauri commands, UI, collectors, detector output, or weekly metrics. `config/aliases.yml` remains the deterministic entity-detector input.

### entity_aliases

Durable aliases for canonical entities with their origin and matching context.

- Primary key: `entity_alias_id` UUID.
- Foreign key: `entity_id` references `canonical_entities`.
- Uniqueness: `(entity_id, normalized_alias, source_scope)`. `normalized_alias` is intentionally not globally unique.
- Source scopes: `global`, `threads`, `explainx`, `github`, `hacker_news`, and `product_hunt`.
- Provenance values: `bootstrap_yaml`, `candidate_review`, `source_review`, and `manual`.
- `is_ambiguous` and optional `context_terms_json` preserve contextual disambiguation requirements.
- Status values are `active` and `archived`; lookup returns active aliases and may return multiple candidate entities.

The explicit curated bootstrap reads the real `config/aliases.yml`, creates a canonical entity only when no normalized-name candidate exists, reuses exactly one candidate, and abstains when multiple candidates exist. It is idempotent and is not run during database initialization.

### external_identity_reviews

Append-only service-level history of explicit decisions about a `source_record_entity_links` relation.

- Primary key: `review_id` UUID.
- Each row snapshots `link_id`, `source_record_id`, `entity_id`, proposed relationship, match evidence, reviewer, and timestamps.
- Decisions are `approved`, `rejected`, or `ambiguous`; pending is represented only by the effective link state.
- Repository operations expose append and chronological read only. Existing audit rows are never updated by the review service.
- Audit insertion, effective link update, review timestamp, and source-record resolution reconciliation occur in one DuckDB transaction.
- Confidence remains diagnostic and never auto-approves a relationship.

DuckDB currently rejects updates to a parent row referenced by a foreign key, even when the parent key itself is unchanged. Because the review transaction must append an audit row before updating the effective link and source-record resolution, this audit table intentionally has no database foreign-key constraints. The repository first loads the link and copies its link, source-record, and entity IDs inside the same transaction, preserving referential integrity at the service boundary. The `entity_aliases` foreign key is unaffected.

## Assumptions

- Raw, normalized, and aggregated data stay separate for auditability.
- External source records, observations, and canonical identities remain separate from social post storage.
- Persistent aliases coexist with YAML detector configuration; they do not replace detector reads.
- Candidate Review (`entity_review_decisions`) and external source identity review (`external_identity_reviews`) answer different questions and remain separate.
- Schema initialization uses `CREATE TABLE IF NOT EXISTS` for MVP.
- Schema initialization uses additive `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` migrations. If a real legacy `agent_mentions_compatible` table or view is present, initialization removes that compatibility object before applying the current schema.
- Phantom compatibility metadata that DuckDB does not expose as a real object is never repaired by deleting the database automatically. Local demo reset returns a friendly local-only cleanup instruction instead.
- A fuller migration system should be introduced only when schema changes become frequent or data migration becomes risky.
- The Apify connector is an experimental fallback. Its extra source metadata is additive and should be reviewed for compliance before production use.
- Apify applies an entity-first gate before raw storage. Generic AI/MCP context is not sufficient; a known concrete alias or strict product-like unknown candidate must be detected. Recruitment/job posts are filtered unless a concrete named entity is present.
- Apify enforces the actor's minimum of 10 max posts. Its synchronous run timeout defaults to 300 seconds and is configurable with `APIFY_RUN_TIMEOUT_SECONDS` within a 30-900 second bound.
- Local demo reset clears `threads_posts_raw`, `agent_mentions`, `weekly_agent_metrics`, `crawl_runs`, and optional `crawl_seed_results` while preserving `entity_review_decisions`.
