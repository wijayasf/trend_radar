# Latest Handoff

Date: 2026-07-24
Session: 035-apify-data-quality
Agent: Codex

## Current State

Apify fallback connector and data-quality hardening have been implemented locally. Official Threads API connector remains in place and is still the default source.

## Key Changes

- Apify fallback source:
  - command: `run_apify_discovery_crawl(seeds?, max_per_seed?)`
  - actor: `futurizerush/meta-threads-scraper`
  - source label: `apify_threads_scraper`
- Apify relevance filter:
  - excludes empty text
  - excludes no AI/developer context
  - excludes Ponytail/Caveman/Cavemen without strong AI/developer context
  - dedupes by `post_code`, fallback `post_url`
  - returns `included_by_context_count` plus filter reason counts
- Candidate extraction:
  - common capitalized words are hard-blocked
  - unknown candidates require tool-ish tokens, product/tool-like phrase shape, or nearby tool/developer context
  - known aliases still work before candidate extraction
- Weekly metrics:
  - groups by lower-trimmed canonical entity key plus week/category/region
- Cost classifier:
  - recognizes more negative/boros, positive, and neutral cost mention patterns
  - `$100/mo` and `subscription` no longer classify as `not_mentioned`
- Local demo reset:
  - command: `reset_local_pipeline_data()`
  - UI button: `Clear Local Demo Data`
  - clears raw posts, mentions, crawl runs, and weekly metrics
  - preserves `entity_review_decisions`

## Validation

- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check`: passed.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.
- `cargo test validates_raw_post_insert_after_schema_init -- --test-threads=1`: passed.
- `cargo test filters_apify_threads_results_for_ai_agent_relevance -- --test-threads=1`: passed.
- `cargo test excludes_common_capitalized_words_from_unknown_candidates -- --test-threads=1`: passed.
- `cargo test keeps_known_aliases_while_tightening_candidates -- --test-threads=1`: passed.
- `cargo test services::cost_classifier::tests -- --test-threads=1`: passed.
- `cargo test validates_weekly_metrics_group_canonical_entities -- --test-threads=1`: passed.
- `cargo test validates_reset_local_pipeline_data_preserves_candidate_decisions -- --test-threads=1`: passed.
- `git diff --check`: passed.
- Security grep for `APIFY_TOKEN=`, `THAAP`, `app_secret`, and `THREADS_ACCESS_TOKEN=`: passed.

## Pending

- Commit locally with `fix: improve Apify discovery data quality`.
- Do not push unless explicitly requested.

## Risk Note

- Candidate extraction is intentionally conservative; add explicit aliases for niche tools that are wrongly excluded.
- Cost neutral mentions map to `cost_mixed` because the current MVP schema has no `cost_mentioned_neutral` label.
- Apify fallback remains experimental and requires compliance review before production use.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
