# Progress Report - Session 034 Apify Fallback Connector

Date: 2026-07-24
Agent: Codex

## Objective

Add an experimental Apify Threads fallback connector using `futurizerush/meta-threads-scraper`, with relevance filtering before raw posts enter the AI Agent Trend Radar pipeline.

## Completed

- Added backend Apify connector service and Tauri command:
  - `run_apify_discovery_crawl(seeds?, max_per_seed?)`
- Added Apify environment placeholders to `.env.example`:
  - `APIFY_TOKEN`
  - `APIFY_THREADS_ACTOR_ID`
  - `APIFY_SOURCE_MODE`
- Added additive DuckDB raw post metadata columns for source audit:
  - `author_display_name`
  - `source_type`
  - `source_seed_keyword`
  - `keyword_match`
  - `share_count`
  - `view_count`
- Mapped Apify fields into `threads_posts_raw` while preserving the original Apify item in `raw_json`.
- Added relevance filter for AI Agent context:
  - Includes AI Agent / Claude Code / MCP / coding workflow context.
  - Excludes empty text.
  - Excludes generic posts without AI context.
  - Excludes ambiguous Ponytail/Caveman/Cavemen mentions without AI Agent context.
  - Dedupes by `post_code`, with `post_url` fallback.
- Added UI source selector in Discovery:
  - Official Threads API
  - Apify fallback
  - Sample/mock
- Added Apify diagnostics in UI:
  - actor ID
  - actor run ID
  - fetched
  - filtered out
  - saved
  - filter reasons
  - saved post snippets
- Updated README and DuckDB schema docs with experimental fallback notes.

## Validation

- `cargo check`: passed.
- `npm run build`: passed.
- `cargo test filters_apify_threads_results_for_ai_agent_relevance -- --test-threads=1`: passed.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.
- `cargo fmt --check`: passed.
- `git diff --check`: passed.
- Security grep for `APIFY_TOKEN=`, `THAAP`, and `app_secret`: passed.
- `src-tauri/data`: no runtime database files found.

## Sample Filter Result

Targeted test confirms:

- `AI Agent roadmap for developer workflow`: included.
- `Claude Code subscription is useful for coding automation`: included.
- `Ponytail feels useful for Claude Code workflow`: included.
- `Who can do a ponytail braid?`: excluded as ambiguous without AI context.
- `Captain Caveman is on TV again`: excluded as ambiguous without AI context.
- Empty text: excluded.
- Generic lifestyle text: excluded.
- Duplicate `post_code`: skipped.

## Risk Note

- Apify fallback is experimental and must be reviewed for legal/compliance fit before production use.
- Actor input schema was implemented with the manually validated search semantics; if the actor changes its schema, the connector may need a small adapter update.
- Apify sync actor response may not always expose a run ID header; UI safely displays `unavailable_sync_run` when absent.

## Next Recommended Task

Run one local Apify smoke test with a configured `APIFY_TOKEN`, then run entity detection and weekly metrics on the saved fallback posts.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
