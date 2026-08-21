# Session 056 - IMP-07 Implementation Brief

Date: 2026-08-21
Agent: Codex
Branch: `planning/imp-07-cross-source-score-prototype`

## Objective

Translate the merged cross-source scoring design and calibration oracle into an explicit, documentation-only implementation brief before IMP-07 coding begins.

## Completed

- Created a fresh planning branch from synchronized `main` at merge commit `e41d1b7`.
- Reviewed DESIGN-01, CAL-01, the eight-entity calibration fixture, current DuckDB schema documentation, and the latest handoff.
- Defined the additive `cross_source_entity_scores` proposal and its versioned, idempotent rebuild boundary.
- Defined trusted ranking persistence separately from watchlist, needs-review, and excluded diagnostics.
- Defined the service, command, read-only UI, fixture tests, trust gates, risk controls, and safe implementation order.
- Preserved all existing application, schema, collection, review, and report behavior.

## Key Decisions

- A current `weekly_entity_metrics` row and resolved active canonical identity are mandatory for trusted ranking.
- Only approved ExplainX `same_entity` links may contribute registry evidence.
- Registry-only evidence remains watchlist-only and creates no score row.
- Non-ranked labels are returned as diagnostics rather than persisted as false score rows.
- Indonesia and Global normalization remain independent.
- `cross-source-v1-proposal` remains the formula contract; later changes require a new version.
- Production recency decay, momentum, Programming Fit, report changes, and live source work remain out of scope.

## Files

- Added `docs/design/2026-08-imp-07-cross-source-score-prototype-brief.md`.
- Added this progress report.
- Updated the latest handoff.
- Updated the token usage log.

## Validation

- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check --locked`: passed with seven unchanged dead-code warnings.
- `cargo test --locked`: passed, 86 passed / 0 failed / 1 intentionally ignored live Threads test.
- `git diff --check`: passed.
- No live Threads, Apify, or ExplainX call ran.

## Security Result

- Requested tracked-file secret scan found no real secret values.
- No `.env`, token value, runtime database, cache, export, frontend build output, or Rust target artifact was intentionally read or added.

## Scope Status

IMP-07 implementation has not started. No application code or runtime schema was changed.

## Risks

- Sparse cohorts, registry caps, and production recency remain future calibration concerns.
- The next implementation session must preserve score-row abstention for watchlist and review-blocked cases.
- JSON explanations must be generated from the same typed factors used for persisted numeric values.

## Recommended Next Step

Review and approve this brief. After approval, create a separate implementation branch and follow the documented order beginning with additive schema and pure fixture-based factor tests.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex runtime token accounting unavailable
- Accuracy: Low
