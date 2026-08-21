# Session 055 - Cross-source Scoring Calibration

Date: 2026-08-21
Agent: Codex
Branch: `design/cross-source-scoring-momentum`

## Objective

Create a deterministic documentation fixture and expected ranking contract before IMP-07 implementation.

## Calibration Result

- Added synthetic resolved, registry-only, pending, ambiguous, missing-alias, and no-product examples.
- Defined four expected labels: trusted ranking, watchlist, needs review, and excluded from score.
- Defined region-local factor normalization and expected rounded scores for trusted rows.
- Set expected Global order to Claude Code, Ponytail, then Caveman.
- Set Claude Code as the only trusted Indonesia row.
- Kept FlowPilot registry-only on the watchlist and kept NovaForge, Codex, and UnknownNewTool outside trusted scoring.
- Added ten minimum IMP-07 acceptance criteria and twelve fixture assertions.

## Scope Boundaries

- Documentation and synthetic JSON fixture only.
- No application code, schema, scoring service, UI, report, or collector change.
- No momentum, WoW, velocity, Programming Fit, LLM scoring, live API request, fuzzy merge, or automatic identity approval.
- IMP-07 has not started.

## Validation

- Calibration JSON parse, count-total checks, factor recomputation, tolerance checks, and expected ranking check: passed for 8 entities.
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

- The fixture intentionally locks a prototype normalization contract; changing it requires a new score version and expected output.
- Production recency decay and large-cohort normalization remain open calibration questions.
- Registry boost must remain conversation-gated to prevent listed-only entities from entering trusted rankings.

## Recommended Next Step

Review CAL-01 and approve or revise its factor formulas, expected ranks, and labels. Start IMP-07 only after this fixture is accepted as the deterministic test oracle.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex runtime token accounting unavailable
- Accuracy: Low
