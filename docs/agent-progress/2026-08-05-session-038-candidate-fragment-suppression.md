# Progress Report - Session 038 Candidate Fragment Suppression

Date: 2026-08-05
Session: 038-candidate-fragment-suppression
Agent: Codex

## Objective

Remove known-alias fragments and generic model concepts from Candidate Review, harden Apify timeout/minimum behavior, and handle legacy DuckDB reset metadata without deleting user data.

## Changes Made

- Suppressed unknown candidates that are sub-fragments of matched known aliases or single-token fragments of any configured multi-token alias.
- Added concept exclusions for `LAM`, `LAMs`, `LLM`, `LLMs`, `CLI`, `Code`, `GitHub`, `Copilot`, `SDK`, and related generic phrases while preserving complete known aliases.
- Added real-snippet regression tests for `Claude Code`, `Codex CLI`, `GitHub Copilot`, Large Action Models, `Graphify`, and `Headroom` behavior.
- Enforced Apify's minimum of 10 max posts in the backend and desktop UI.
- Increased the Apify synchronous timeout default from 90 to 300 seconds, added bounded `APIFY_RUN_TIMEOUT_SECONDS` configuration, and added a friendly timeout message.
- Added typed cleanup for a real legacy `agent_mentions_compatible` table/view and a friendly local-only reset instruction for phantom metadata. No database file is deleted automatically.
- Corrected two pre-existing entity tests that depended on result ordering when their input contained multiple known entities.

## Real-Data Replay

- Previous baseline: 42 raw posts, 103 mentions, 70 pending candidates.
- Entity-first gate before this patch: 16 included posts, 22 mentions, 4 pending candidates, 9 weekly rows.
- After fragment suppression: 15 included posts, 17 mentions, 0 pending candidates, 9 weekly rows.
- Removed false positives: `Code`, `CLI`, `GitHub`, and `LAMs`.
- Weekly rows remained canonical: `Claude Code`, `Cursor`, `Caveman`, `Cline`, `Codex CLI`, `Ponytail`, `ExplainX`, `GitHub Copilot`, and `OpenCode`.
- Standalone `MCP` remained excluded from weekly ranking.

## Validation

- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check`: passed with existing placeholder/dead-code warnings only.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.
- `cargo test validates_raw_post_insert_after_schema_init -- --test-threads=1`: passed.
- All 20 entity-detector tests: passed.
- All Apify connector tests, including minimum and timeout normalization: passed.
- Candidate-decision-preserving reset test: passed.
- Legacy compatibility table/view cleanup test: passed.
- Weekly canonical grouping and generic MCP exclusion test: passed.
- Real 42-post Apify replay through a temporary DuckDB: passed with zero pending candidates.
- `src-tauri/data` remained empty.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low

## Risks / Notes

- The 300-second Apify timeout path is unit-normalized and build-validated but was not exercised with another paid/live actor run in this session.
- The root local DuckDB may still contain phantom catalog metadata. The app now explains the manual local-only cleanup option; it does not remove the database.
- The fragment rule is conservative: a standalone token that is part of a configured multi-token alias cannot become an unknown candidate.

## Next Recommended Task

Perform one fresh Apify UI crawl with the 300-second timeout and review included/filtered samples. If operational behavior is healthy, push the three local quality commits.
