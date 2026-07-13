# Latest Handoff

Date: 2026-07-13
Session: 023-discovery-crawler-mvp
Agent: Codex

## Current State

The app now has a discovery-first MVP flow:

1. Run AI Agent Discovery Crawl
2. Detect Agent Mentions
3. Classify Regions
4. Classify Sentiments
5. Classify Cost Signals
6. Aggregate Weekly Metrics
7. Export Markdown Report
8. Export CSV Metrics

The manual Threads keyword collector remains available for debugging one keyword, but discovery crawl is now the intended research entry point.

## Completed

- Added `config/discovery_keywords.yml`.
- Added `discovery_crawler` service and `run_discovery_crawl` command.
- Added UI section for AI Agent Discovery Crawler.
- Added sample/mock fallback when real Threads search is unavailable.
- Added in-crawl deduplication by Threads post ID.
- Added safe diagnostic for ID-only keyword search responses.
- Updated sample posts to discovery-style AI Agent posts.
- Added aliases for Astryx, `astryx.ai`, `ponytail.dev`, and `caveman coding`.
- Added candidate entity extraction with:
  - `category = unknown_candidate`
  - `detection_source = candidate_pattern`
  - `needs_review = true`
- Added known alias metadata:
  - `detection_source = known_alias`
  - `needs_review = false`
- Added compatibility migration for `agent_mentions`.
- Updated README and DuckDB schema docs.
- Updated targeted full-flow test to start from discovery crawl.

## Validated Counts

Sample/mock discovery full-flow:

- Raw posts: `10`
- Weekly report total mentions: `18`
- Total agents detected: `11`
- Regions covered: `global`, `indonesia`
- Known entities confirmed:
  - `Ponytail`
  - `Caveman`
  - `Astryx`
  - `Claude Code`
  - `Cursor`
  - `MCP`
  - `Cline`
  - `Codex CLI`
- Candidate entity confirmed:
  - `NovaForge`
- Sentiment:
  - Positive: `11`
  - Neutral: `7`
  - Negative: `0`
  - Mixed: `0`
- Cost:
  - Not mentioned: `13`
  - Cost positive: `3`
  - Cost negative / boros: `2`
  - Cost mixed: `0`

## Validation Commands

- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check`: passed.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.
- `KEEP_REPORT_EXPORTS=1 cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.
- `src-tauri/data`: no runtime database files.

## Known Warnings

- Existing Rust warnings remain for placeholder trend models and placeholder Threads trait.
- Threads real API still requires `threads_keyword_search` permission.
- ID-only keyword search detail fetch is not implemented yet.
- Candidate extraction is rule-based and can produce false positives.

## Suggested Next Step

Build a candidate review workflow for `unknown_candidate` mentions, then add post detail fetch for ID-only Threads keyword search responses once the official permission/endpoint behavior is available.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
