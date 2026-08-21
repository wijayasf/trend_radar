# Session 054 - Cross-source Scoring Design

Date: 2026-08-21
Agent: Codex
Branch: `design/cross-source-scoring-momentum`

## Objective

Define an auditable design for future cross-source scoring and momentum before any IMP-07 implementation.

## Design Result

- Separated Threads/Apify conversation meaning from ExplainX registry/discovery meaning.
- Made explicit human-approved `same_entity` links an eligibility gate for registry contribution.
- Defined normalized score dimensions and a proposed weighted formula with visible factor breakdown.
- Kept Indonesia, Global, and Unknown as separate regional cohorts.
- Defined future WoW and momentum semantics while excluding them from IMP-07.
- Scoped IMP-07 to an additive score prototype table and read-only UI preview.

## Scope Boundaries

- Documentation only; no application code or schema was changed.
- No cross-source score, momentum, WoW, velocity, Programming Fit, LLM classifier, live ExplainX collector, fuzzy merge, or report replacement was implemented.
- Existing `weekly_agent_metrics`, `weekly_entity_metrics`, identity review semantics, and collector behavior remain unchanged.

## Validation

- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check --locked`: passed with seven unchanged dead-code warnings.
- `cargo test --locked`: passed, 86 passed / 0 failed / 1 intentionally ignored live Threads test.
- `git diff --check`: passed.
- No live Threads, Apify, or ExplainX request ran.

## Security Result

- Requested Apify token, Threads token, THAAP, and app-secret scans found no real secret values.
- Matches were limited to CI detection patterns and historical documentation naming those patterns.
- `.env`, local databases, cache, exports, `dist`, `node_modules`, and `src-tauri/target` remained outside the change.

## Risks

- Weights and normalization require fixture calibration before implementation.
- ExplainX listing presence can over-rank listed-but-undiscussed tools unless the conversation-row gate remains mandatory.
- Sparse Indonesia data needs regional normalization and evidence-count display.
- Momentum requires reliable consecutive weekly coverage and must distinguish no observation from confirmed zero results.

## Recommended Next Step

Review and approve the design. Before IMP-07 code begins, agree on a calibration fixture and expected factor breakdown for discussed-only, listed-only, discussed-and-listed, ambiguous, rejected, and sparse-Indonesia cases.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex runtime token accounting unavailable
- Accuracy: Low
