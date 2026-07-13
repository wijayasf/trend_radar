# Progress Report: Session 023 - AI Agent Discovery Crawler MVP

Date: 2026-07-13
Agent: Codex

## Objective

Realign the app from a single manual keyword workflow to a discovery-first AI Agent research flow:

```text
Run AI Agent Discovery Crawl
→ collect raw posts from multiple AI Agent seed keywords
→ detect agent/tool/skill entities
→ classify region, sentiment, and cost/boros
→ aggregate weekly metrics
→ export Markdown/CSV report
```

This session intentionally did not change the report exporter logic beyond integration validation and did not print token or `.env` contents.

## Completed

- Added `config/discovery_keywords.yml` with global and Indonesia AI Agent seed keywords.
- Added discovery crawler service:
  - `src-tauri/src/services/discovery_crawler.rs`
- Added Tauri command:
  - `run_discovery_crawl`
- Added UI section:
  - `AI Agent Discovery Crawler`
  - seed group selector: all, Indonesia, global
  - max per seed input
  - discovery summary counters and safe diagnostics
- Kept the manual Threads keyword collector for single-keyword debugging.
- Added sample/mock fallback when real Threads discovery is unavailable.
- Added raw post deduplication by Threads post ID within each crawl.
- Added safe diagnostic for ID-only keyword search responses:
  - `Keyword search returned IDs only; post detail fetch is required for text-based entity detection.`
- Updated sample posts to discovery-style AI Agent topic posts.
- Added known aliases:
  - `Astryx`
  - `astryx.ai`
  - `ponytail.dev`
  - `caveman coding`
- Extended MCP context terms for discovery-style posts.
- Added candidate entity extraction MVP:
  - capitalized/domain-like names in AI/coding/agent context
  - category `unknown_candidate`
  - detection source `candidate_pattern`
  - `needs_review = true`
- Added `detection_source` and `needs_review` to `agent_mentions`.
- Added compatibility migration for existing `agent_mentions` tables so `unknown_candidate` is accepted.
- Updated UI mention preview to show detection source and review state.
- Updated README and DuckDB schema docs.
- Updated targeted full-flow test to start from discovery sample/mock flow.

## Validation Counts

Targeted full-flow validation with sample/mock discovery:

- Raw posts: `10`
- Total mentions in weekly report: `18`
- Total agents detected: `11`
- Regions covered: `global`, `indonesia`
- Known aliases detected:
  - `Ponytail`
  - `Caveman` from `Cavemen`
  - `Astryx`
  - `Claude Code`
  - `Cursor`
  - `MCP`
  - `Cline`
  - `Codex CLI`
- Candidate entity detected:
  - `NovaForge` as `unknown_candidate`
- Sentiment overview:
  - Positive: `11`
  - Neutral: `7`
  - Negative: `0`
  - Mixed: `0`
- Cost overview:
  - Not mentioned: `13`
  - Cost positive: `3`
  - Cost negative / boros: `2`
  - Cost mixed: `0`

## Validation

- `npm run build` passed.
- `cargo fmt --check` passed.
- `cargo check` passed.
  - Existing placeholder warnings remain for unused trend models and placeholder Threads trait.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1` passed.
- `KEEP_REPORT_EXPORTS=1 cargo test validates_sample_full_mvp_flow -- --test-threads=1` passed and produced updated Markdown/CSV sample exports.
- Confirmed `src-tauri/data` has no runtime database files.

## Exported Sample Files

- Markdown: `data/exports/weekly-report-2026-06-29-to-2026-07-05.md`
- CSV: `data/exports/weekly-metrics-2026-06-29-to-2026-07-05.csv`

These are ignored runtime exports.

## Not Completed

- Real Threads keyword search remains blocked until `threads_keyword_search` permission is available.
- Post detail fetch for ID-only keyword search responses is not implemented yet.
- Candidate review/approval UI is not implemented yet.
- DOCX/PDF export remains out of scope.

## Risk Note

- Candidate extraction is intentionally simple and may still produce false positives.
- Sample/mock fallback is useful for development but should be clearly separated from real Threads crawling in user interpretation.
- The compatibility migration recreates `agent_mentions` to update the category constraint while preserving rows.
- No token or `.env` contents were read or printed.

## Next Recommended Task

Add a candidate review workflow so `unknown_candidate` entities can be approved into `config/aliases.yml` or marked ignored before they influence long-term reports.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
