# Session 058 - PR 5 Self-Review

Date: 2026-08-21
Agent: Codex
PR: https://github.com/wijayasf/trend_radar/pull/5
Branch: `feature/imp-07-cross-source-score-prototype`
Base: `main`

## Objective

Perform a review-only assessment of the IMP-07 Cross-source Score Prototype after GitHub CI completed. No application code was changed, PR #5 was not merged, and IMP-08 was not started.

## Commits Reviewed

- `6205009 feat: add cross-source score prototype`
- `dbbd872 docs: finalize IMP-07 checkpoint`

The PR contains 14 changed files with 2,293 additions and 19 deletions relative to `origin/main`.

## CI Result

All six checks passed across the push and pull-request workflow runs:

- Frontend build: passed twice.
- Rust validation: passed twice.
- Tracked secret scan: passed twice.

PR #5 remains open, draft, and merge-clean.

## Findings

No blocking correctness, persistence, security, regression, or scope finding was identified.

### Schema Review

- `cross_source_entity_scores` is additive and initialized with `CREATE TABLE IF NOT EXISTS`.
- The unique bucket is `(score_version, week_start, entity_id, region)`.
- Region, ranking label, factor ranges, adjustments, and final-score bounds are constrained.
- Rebuild deletes only the selected score-version/week bucket inside a transaction, preserves historical weeks, and rolls back on failure.
- Existing `weekly_entity_metrics` and `weekly_agent_metrics` schemas and aggregation behavior are not replaced or rewritten.
- The only new reset behavior deletes the derived cross-source rows with the other local demo-derived data.

### Scoring And Oracle Review

- Formula version is `cross-source-v1-proposal` and matches the merged calibration formula.
- Mention normalization is independent per week and region; Indonesia and Global remain separate.
- Conversation evidence remains the main driver. Registry and review factors are binary support signals, and multiple approved records do not multiply those factors beyond 100.
- The fixture verifies Global order `Claude Code > Ponytail > Caveman` and Indonesia containing only Claude Code.
- FlowPilot remains watchlist-only. NovaForge, Codex, and UnknownNewTool remain needs-review. Generic MCP editorial evidence remains excluded.
- Pending Ponytail registry evidence contributes zero registry/review factors. Approved Claude Code `same_entity` evidence contributes both factors.

### Registry Gating Review

- Trusted score inputs come from current `weekly_entity_metrics` rows joined to active canonical entities.
- Only active ExplainX records with an approved `same_entity` effective link contribute registry evidence.
- Pending and ambiguous links contribute no registry or review-confidence factor.
- Rejected and unsupported relationship evidence is excluded from registry contribution and surfaced diagnostically where relevant.
- Registry-only entities cannot create score rows; approved registry evidence without conversation is returned as watchlist.
- Candidate Review and External Identity Review behavior remain unchanged.

### Persistence Boundary Review

- Only `trusted_ranking` rows are persisted.
- Watchlist, needs-review, and excluded results are command diagnostics and create no score rows.
- Re-running the same version/week is idempotent and does not create duplicate rows.
- A failed replacement rolls back to the prior score set.
- Historical score weeks survive latest-week rebuilds.
- `factor_breakdown_json` contains normalized factors and resulting component scores. The row also persists raw count columns and score version.
- `source_evidence_json` contains counts, approved non-secret record keys, and freshness timestamps. It contains no access token or raw post/personal content.

### UI Review

- The new panel is read-only except for the explicit aggregate action.
- Top Indonesia and Top Global are separate tables.
- Watchlist, needs-review, and excluded diagnostics remain outside ranking tables.
- Score, factors, adjustments, evidence explanation, version, week, and counts are visible.
- Existing report export handlers and output behavior are unchanged.

## Test Coverage Mapping

1. Calibration ranking and numeric oracle: `calibration_fixture_matches_approved_oracle`.
2. Claude Code above Ponytail and Caveman: fixture oracle plus `fixture_backed_rebuild_is_idempotent_and_preserves_weekly_tables`.
3. Indonesia contains only Claude Code: fixture oracle plus fixture-backed rebuild.
4. FlowPilot watchlist and not ranked: fixture oracle plus fixture-backed rebuild.
5. NovaForge, Codex, and UnknownNewTool needs-review: fixture oracle plus fixture-backed diagnostics.
6. Generic MCP editorial excluded: fixture oracle plus fixture-backed diagnostics.
7. Registry-only evidence cannot rank: FlowPilot score-row and top-list assertions.
8. Pending ExplainX evidence gives no boost: Ponytail registry/review factor assertions.
9. Approved `same_entity` contributes registry score: Claude Code registry factor assertion and numeric fixture factors.
10. `weekly_entity_metrics` unchanged: before/after compatibility snapshot.
11. `weekly_agent_metrics` unchanged: before/after compatibility snapshot.
12. Idempotent rebuild and no duplicates: two rebuilds with an exact persisted-row count assertion.
13. Rollback safety: `failed_score_rebuild_rolls_back_previous_rows`.
14. Parallel safety: default `cargo test --locked` passed.
15. Serial safety: `cargo test --locked -- --test-threads=1` passed.

Additional coverage verifies additive initialization on legacy data and preservation of historical score weeks.

## Scope Boundaries

The PR does not implement momentum, WoW, velocity, rank change, Programming Fit, LLM scoring, live ExplainX scraping, fuzzy merge, automatic identity approval, weekly table replacement, or report export changes. No live Threads, Apify, or ExplainX call ran during review.

## Validation

- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check --locked`: passed with seven unchanged dead-code warnings.
- `cargo test --locked`: passed, 91 passed / 0 failed / 1 intentionally ignored live Threads test.
- `cargo test --locked -- --test-threads=1`: passed, 91 passed / 0 failed / 1 intentionally ignored live Threads test.
- `git diff --check`: passed.

## Security Result

- Requested tracked-file scans found no real Apify token, Threads token, THAAP token, or app secret.
- Matches were limited to CI patterns and historical documentation naming those patterns.
- `.env`, DuckDB runtime data, cache, exports, `dist`, `node_modules`, and Rust target output remain ignored and untracked.

## Non-Blocking Notes

- `factor_breakdown_json` contains the numeric factors and scores but does not repeat the weight map or formula version inside the JSON object. The score version is persisted in its own column and the approved weights remain documented and fixture-locked. A future score-version revision should make this JSON self-contained.
- `source_evidence_json` stores approved record keys, counts, and timestamps but does not repeat explicit surface names or effective review state. Eligibility is enforced by the approved ExplainX `same_entity` query. Adding those labels later would improve audit readability without changing score semantics.
- Production recency decay remains intentionally uncalibrated at 100 for the current-week prototype.
- Source diversity relies on count-level canonical weekly evidence and does not retain individual conversation source names.

## Review Conclusion

`APPROVE AS DRAFT CHECKPOINT`

IMP-07 matches the approved ranking oracle, keeps trust gates and persistence boundaries intact, preserves existing weekly/report behavior, and is covered by deterministic parallel-safe tests. This conclusion approves continued human review of the draft; it is not approval to merge or begin IMP-08.

## Recommended Next Step

Keep PR #5 draft for human review of factor explainability, evidence traceability, and UI clarity. Resolve review feedback before deciding whether to mark the PR ready. Do not begin momentum, WoW, velocity, Programming Fit, or IMP-08 work.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex runtime token accounting unavailable
- Accuracy: Low
