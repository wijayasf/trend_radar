# Progress Report - Session 035 Apify Data Quality

Date: 2026-07-24
Agent: Codex

## Objective

Improve data quality for Apify-backed discovery before adding new features.

## Completed

- Expanded Apify relevance filter for AI/developer context:
  - Added `LLMs`, `plugin`, `SDK`, `CLI`, `server`, `framework`, `model`, and `tool`.
  - Added `included_by_context_count` diagnostic.
  - Confirmed ambiguous Ponytail/Caveman/Cavemen results require AI/developer context.
- Tightened unknown candidate extraction:
  - Added hard stopword block for common capitalized words such as `And`, `The`, `Any`, `But`, `Here`, `Good`, `I'm`, and `APIs`.
  - Unknown candidates now require tool-ish tokens, product/tool-like phrase shape, or meaningful nearby tool/developer context.
  - Known aliases still run before candidate extraction.
- Fixed weekly metrics duplicate rows:
  - Weekly aggregation now groups by lower-trimmed canonical entity key plus week/category/region.
- Improved cost classifier:
  - Added more boros/expensive/token/quota patterns.
  - Added positive value patterns such as `sepadan` and `worth the money`.
  - Added neutral cost mention detection for `subscription`, `$100/mo`, `per month`, pricing/plan/paid terms.
  - Neutral cost mentions map to `cost_mixed` instead of `not_mentioned`.
- Added local demo reset:
  - New command `reset_local_pipeline_data()`.
  - UI button `Clear Local Demo Data`.
  - Clears raw posts, mentions, crawl runs, and weekly metrics.
  - Preserves durable candidate review decisions.

## Validation

Final validation passed:

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

## Before / After Quality

Before:

- 42 Apify raw posts produced 103 mentions and 70 pending candidates.
- Common words entered Candidate Review.
- Weekly metrics showed duplicate canonical entities.
- Cost mentions like `$100/mo` and `subscription` could be missed.

After expected:

- Ambiguous Apify posts without AI/developer context are filtered before raw storage.
- Candidate Review should show fewer, more meaningful pending candidates.
- `Claude Code`, `MCP`, and `Cursor` should aggregate into one row per region/week/category.
- Subscription and monthly price language should no longer be `not_mentioned`.

## Risk Note

- Candidate extraction is now conservative by design; some niche one-word tools may require approval through explicit aliases or stronger context.
- Cost neutral mentions are mapped to `cost_mixed` because the current MVP label set has no dedicated `cost_mentioned_neutral`.
- Apify fallback remains experimental and requires compliance review before production use.

## Next Recommended Task

Run a real Apify crawl after reset, then measure raw posts, mentions, pending candidates, and weekly rows to confirm quality improvement on live data.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
