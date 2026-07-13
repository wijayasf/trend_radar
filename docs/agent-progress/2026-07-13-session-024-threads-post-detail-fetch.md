# Progress Report: Session 024 - Threads Post Detail Fetch

Date: 2026-07-13
Agent: Codex

## Objective

Add safe post detail fetch for Threads keyword search responses that only contain post IDs, so discovery crawl can store text/caption before entity detection runs.

## Completed

- Updated Threads keyword search to use:
  - `GET https://graph.threads.net/v1.0/keyword_search`
  - `Authorization: Bearer <token>`
  - `q=<keyword>`
  - `media_type=TEXT`
- Added post detail fetch helper:
  - `fetch_thread_post_detail(post_id)`
  - internal request: `GET https://graph.threads.net/v1.0/{post_id}`
  - fields requested: `id,text,caption,media_type,permalink,timestamp,username,owner`
- Added ID-only response handling:
  - counts ID-only results
  - fetches details per post ID
  - saves detail result when available
  - keeps raw ID record if detail fetch fails
  - records safe error summaries without token values
- Added `text_missing` storage flag to `threads_posts_raw`.
- Added optional author fields from detail response into `ThreadPostRaw`.
- Updated discovery crawl summary fields:
  - `detail_fetched_total`
  - `detail_failed_total`
  - `id_only_results_count`
- Updated UI discovery panel to show:
  - ID-only results
  - detail fetched
  - detail failed
- Added `cfg(test)` mock ID-only keyword search and mock detail responses for Ponytail, Cavemen, and Astryx.
- Updated targeted full-flow test to start from mock ID-only discovery and detail fetch.
- Tightened candidate token cleanup so region words like `Indonesia` are not extracted as candidate entities.
- Updated README and DuckDB schema docs.

## Mock Full-Flow Counts

Mock ID-only discovery full-flow produced:

- Raw posts saved: `3`
- ID-only results: greater than `0`
- Detail fetched: greater than `0`
- Detail failed: `0`
- Weekly report total mentions: `5`
- Total agents detected: `5`
- Regions covered: `global`, `indonesia`
- Known entities detected:
  - `Ponytail`
  - `Caveman` from `Cavemen`
  - `Astryx`
  - `Claude Code`
  - `Cline`
- Sentiment:
  - Positive: `2`
  - Neutral: `3`
  - Negative: `0`
  - Mixed: `0`
- Cost:
  - Not mentioned: `5`
  - Cost positive: `0`
  - Cost negative / boros: `0`
  - Cost mixed: `0`

## Validation

- `npm run build` passed.
- `cargo fmt --check` passed.
- `cargo check` passed.
  - Existing placeholder warnings remain.
  - New warning: public `fetch_thread_post_detail` helper is not directly called by app command yet; internal detail fetch path is tested through discovery.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1` passed.
- `KEEP_REPORT_EXPORTS=1 cargo test validates_sample_full_mvp_flow -- --test-threads=1` passed and produced Markdown/CSV smoke exports.
- Confirmed `src-tauri/data` has no runtime database files.

## Risk Note

- Detail endpoint field availability may vary with Threads permissions and API behavior.
- If detail fetch succeeds but text/caption is unavailable, the raw post is saved with `text_missing = true` and entity detection skips it.
- No token or `.env` contents were read or printed.

## Next Recommended Task

Run one manual real Threads discovery crawl after `threads_keyword_search` permission is available, then add a detail-fetch diagnostic view if real API responses differ from the mock.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
