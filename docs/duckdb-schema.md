# DuckDB Schema Foundation

This is the MVP local storage boundary for AI Agent Trend Radar. It is intentionally small and can be migrated later when ingestion and reporting requirements become clearer.

## Storage Boundary

- `threads_posts_raw` stores raw Threads post records and source metadata.
- `crawl_runs` stores summary diagnostics for discovery crawl runs.
- `agent_mentions` stores normalized AI agent/tool mentions detected inside raw posts.
- `entity_review_decisions` stores durable approve/ignore decisions for unknown candidates.
- `weekly_agent_metrics` stores report-ready weekly aggregates by agent and region.

No Threads access token, API key, or app secret should ever be stored in DuckDB.

## Tables

### threads_posts_raw

Raw local archive of Threads posts collected for trend analysis.

- `post_id`: Threads post identifier.
- `thread_id`: Optional parent/thread identifier.
- `author_id`: Threads author identifier from API data.
- `author_username`: Display username if available.
- `text`: Post text.
- `text_missing`: `true` when keyword/detail response did not provide text or caption.
- `permalink`: Optional post URL.
- `media_type`: Optional Threads media type from the API response.
- `language`: Optional detected or API-provided language.
- `region_hint`: Optional region hint such as `indonesia`, `global`, or `unknown`.
- `region_confidence`: Rule-based classifier confidence for the post region.
- `region_reason`: Short explainable reason for the post region label.
- Engagement fields: like, reply, repost, and quote counts.
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

Weekly aggregation includes known aliases and approved candidates. Pending `unknown_candidate` rows and ignored candidates are excluded from Top Indonesia/Global/Unknown metrics so unreviewed discoveries do not pollute rankings.

MVP trend score formula:

```text
mentions * 10
+ positive_count * 3
+ mixed_count * 1
- negative_count * 2
- cost_negative_boros_count * 1
```

The score formula should move to `config/scoring.yml` when the ranking design stabilizes.

## Assumptions

- Raw, normalized, and aggregated data stay separate for auditability.
- Schema initialization uses `CREATE TABLE IF NOT EXISTS` for MVP.
- Schema initialization uses additive `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` migrations; there is no `agent_mentions_compatible` table or view.
- A fuller migration system should be introduced only when schema changes become frequent or data migration becomes risky.
