# Progress Report - Session 037 Real Apify Entity Gate Validation

Date: 2026-08-05
Session: 037-real-apify-entity-gate-validation
Agent: Codex

## Objective

Validate the entity-first Apify discovery gate against real data, audit candidate and weekly-metric quality, and decide whether local commits `e0d9b78` and `26b86d3` are ready to push.

## Changes Made

- No application feature changes were retained.
- Replayed the existing 42-post real Apify dataset through the current named-entity gate into a fresh temporary DuckDB.
- Ran entity detection, candidate review listing, region/sentiment/cost classification, and weekly aggregation against only the gated posts.
- Recorded exact gate counts, filter reasons, remaining candidate false positives, and weekly ranking behavior.

## Real Data Result

- A fresh synchronous Apify actor run was attempted with the UI seed set and again with four diagnostic seeds. Both valid requests timed out after 90 seconds before a response was received.
- A temporary `max_posts=3` attempt was rejected safely because the actor requires at least 10 posts.
- Offline replay of the previous real Apify dataset succeeded:
  - Input real posts: 42
  - Entity-gate included: 16
  - Entity-gate filtered: 26
  - Filtered `generic_ai_agent_only`: 10
  - Filtered `generic_mcp_only`: 6
  - Filtered `generic_threadbait`: 1
  - Filtered `no_named_entity`: 9
  - Mentions: 22
  - Pending candidates: 4
  - Approved decisions in fresh validation DB: 0
  - Weekly metric rows: 9
  - Indonesia rows: 0
  - Global rows: 9
  - Unknown rows: 0
- Compared with the prior 42 raw / 103 mentions / 70 pending baseline, the replay reduced mentions by 81 and pending candidates by 66.

## Quality Findings

- Accepted named entities included `Claude Code`, `Cursor`, `Caveman`, `Cline`, `Codex CLI`, `Ponytail`, `ExplainX`, `GitHub Copilot`, and `OpenCode`.
- Remaining pending false positives were `Code`, `CLI`, `GitHub`, and `LAMs`.
- Exact sources were:
  - `Code`: "Claude Code is becoming my default coding agent for repo refactors and test writing."
  - `CLI`: "Codex CLI is useful when I want terminal-first coding agent help without leaving the repo."
  - `GitHub`: "GitHub Copilot still wins for quick autocomplete, but agent workflows are catching up."
  - `LAMs`: a generic explanation that Agentic AI uses Large Language Models and Large Action Models.
- `LAMs` came from a post explaining Large Action Models and violates the requirement to exclude generic concepts such as Large Action Models.
- Generic AI Agent and standalone MCP examples were filtered before storage.
- Weekly metrics excluded all pending candidates and standalone `MCP`.
- `Claude Code` appeared once in the global region/week with 7 mentions; `Cursor` appeared once with 2 mentions.

## Validation

- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check`: passed with existing placeholder/dead-code warnings only.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.
- `cargo test validates_raw_post_insert_after_schema_init -- --test-threads=1`: passed.
- `cargo test entity_gate -- --test-threads=1`: passed.
- `cargo test accepts_strict_brand_like_unknown_candidates -- --test-threads=1`: passed.
- `cargo test excludes_generic_concepts_and_threadbait_fragments_from_candidates -- --test-threads=1`: passed.
- `cargo test excludes_common_capitalized_words_from_unknown_candidates -- --test-threads=1`: passed.
- `cargo test validates_weekly_metrics_group_canonical_entities_and_exclude_generic_mcp -- --test-threads=1`: passed.
- Security grep found only historical documentation references to scan patterns; no token or secret values were found.
- `src-tauri/data` remained empty.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low

## Risks / Notes

- Do not push `e0d9b78` and `26b86d3` yet: candidate quality is much better but still admits four generic false positives, including the explicitly unwanted Large Action Models concept as `LAMs`.
- The synchronous Apify actor did not complete within the current 90-second HTTP timeout, so this session did not obtain a new live dataset.
- The root local DuckDB retains legacy phantom `agent_mentions_compatible` dependency metadata, which blocks `Clear Local Demo Data`. The real dataset was left untouched and validation used `/tmp`.
- Native Tauri click-through could not be completed because the restarted process exposed no accessible window to macOS automation.

## Next Recommended Task

Add focused candidate exclusions for `LAMs`, `Code`, `CLI`, and bare `GitHub`, backed by the exact real snippets from this validation. Then rerun the 42-post replay and address Apify actor polling/timeout and legacy local DB repair as separate narrowly scoped tasks.
