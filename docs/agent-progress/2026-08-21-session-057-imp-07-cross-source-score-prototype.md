# Session 057 - IMP-07 Cross-source Score Prototype

Date: 2026-08-21
Agent: Codex
Branch: `feature/imp-07-cross-source-score-prototype`

## Objective

Implement the smallest safe, explainable cross-source score prototype from the merged DESIGN-01, CAL-01 fixture, and IMP-07 implementation brief.

## Schema Changes

- Added the additive `cross_source_entity_scores` table.
- Preserved `weekly_entity_metrics` and `weekly_agent_metrics` unchanged as input and legacy reporting layers.
- Added unique version/week/entity/region buckets, bounded factors, explanation/evidence JSON, and ranking-label constraints.
- Added version/week-scoped transactional rebuild, historical-week preservation, and local demo reset coverage.
- No destructive migration was added.

## Scoring Behavior

- Formula version: `cross-source-v1-proposal`.
- Uses latest-week resolved active canonical Indonesia/Global conversation rows only.
- Uses approved active ExplainX `same_entity` links only for registry contribution.
- Keeps pending links at zero registry/review-confidence boost.
- Persists only `trusted_ranking` rows.
- Returns registry-only evidence as `watchlist`, unresolved/ambiguous/missing identity as `needs_review`, and rejected/no-product/unsupported/unknown-region evidence as `excluded_from_score` diagnostics.
- Keeps Indonesia and Global normalization independent.

## Fixture Oracle Result

- Global order: Claude Code, Ponytail, Caveman.
- Indonesia: Claude Code only.
- FlowPilot: watchlist with no score row.
- NovaForge, Codex, and UnknownNewTool: needs review with no score row.
- Generic MCP editorial evidence: excluded.
- Pending Ponytail registry evidence contributes no boost.
- Approved Claude Code same-entity evidence contributes registry and review-confidence factors.
- All checked factor values remain within the fixture numeric tolerance.

## UI Behavior

- Added a read-only `Cross-source Score Preview` panel.
- Added explicit aggregate/loading state and score-version/week/count summary.
- Added separate Top Indonesia and Top Global tables with factor and evidence previews.
- Added separate watchlist, needs-review, and excluded diagnostic lists.
- Existing weekly and report export panels remain unchanged.

## Tests Added

- Deterministic calibration fixture formula, label, and regional ranking validation.
- Fixture-backed DuckDB aggregation, registry gating, diagnostics, and idempotency validation.
- Latest-week rebuild preserves historical score rows from earlier weeks.
- `weekly_entity_metrics` and `weekly_agent_metrics` compatibility snapshots.
- Additive legacy-schema initialization validation.
- Failed rebuild rollback validation.
- Test databases use unique scoped paths with no `DATABASE_PATH` environment mutation.

## Validation

- Targeted cross-source tests: passed, 5 passed / 0 failed.
- `npm run build`: passed after UI integration.
- `cargo fmt --check`: passed.
- `cargo check --locked`: passed with seven unchanged dead-code warnings.
- `cargo test --locked`: passed, 91 passed / 0 failed / 1 intentionally ignored live Threads test.
- `cargo test --locked -- --test-threads=1`: passed, 91 passed / 0 failed / 1 intentionally ignored live Threads test.
- `git diff --check`: passed.
- Diff scan confirmed no `DATABASE_PATH`, `set_var`, or `remove_var` mutation was introduced.
- No live Threads, Apify, or ExplainX call ran.

## Security Result

- Requested tracked-file secret scan found no real secret values. Matches were limited to CI patterns and historical documentation naming those patterns.
- No token, `.env`, runtime database, cache, export, `dist`, `node_modules`, or Rust target artifact was intentionally added.

## Explicitly Not Implemented

- Momentum, WoW, velocity, or rank-change scoring.
- Programming Fit.
- LLM scoring or classification.
- Live ExplainX scraping.
- Fuzzy merge or automatic identity approval.
- Replacement or mutation of existing weekly metrics.
- Report export changes.

## Risks

- Production recency decay remains intentionally uncalibrated; the current-week prototype uses `100`.
- Sparse Indonesia cohorts can amplify relative differences, so evidence counts remain visible.
- `weekly_entity_metrics.source_count` supplies count-level source diversity but does not preserve source names; the factor is capped to the approved three-surface contract.
- Non-ranked diagnostics are derived from current effective states and are not historical score rows.

## Recommended Next Step

Commit and push the validated IMP-07 checkpoint, then open a draft PR. Human review should verify the factor explanation and persistence boundaries before any momentum design begins.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex runtime token accounting unavailable
- Accuracy: Low
