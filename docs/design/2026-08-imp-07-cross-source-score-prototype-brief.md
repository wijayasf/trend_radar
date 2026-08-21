# IMP-07 Cross-source Score Prototype Implementation Brief

Date: 2026-08-21
Status: Approved implementation brief; implementation not started
Score version: `cross-source-v1-proposal`

This brief translates the merged DESIGN-01 and CAL-01 documents into a bounded implementation plan. The calibration fixture remains the deterministic oracle. If this brief conflicts with the fixture, the fixture and its documented acceptance criteria take precedence until a new score version is approved.

## 1. Objective

Implement the smallest safe additive cross-source scoring prototype by combining:

- canonical conversation metrics from `weekly_entity_metrics`;
- active canonical identity from `canonical_entities`;
- approved ExplainX `same_entity` evidence;
- independently normalized Indonesia and Global cohorts.

The prototype must be explainable and deterministic. It adds a derived score layer without changing collectors, classifiers, identity semantics, existing weekly metrics, or report exports.

## 2. Data Inputs

### canonical_entities

Provides the stable `entity_id`, display name, type, and active status. Only active canonical entities are eligible for a trusted score row.

### agent_mentions

Provides the audited path from social mentions to canonical identities. Only mentions with a resolved `entity_id` can contribute indirectly through `weekly_entity_metrics`. IMP-07 must not relink mentions or infer identity.

### weekly_entity_metrics

Provides the required current conversation row per canonical entity, week, and region:

- mention and source counts;
- positive, neutral, negative, and mixed sentiment counts;
- cost-positive, cost-negative/boros, cost-mixed, and not-mentioned counts;
- first-seen and last-seen timestamps.

This table remains the authoritative conversation input and must not be modified by score aggregation.

### explainx_records and source_records

Provide active registry metadata, durable source identity, last-seen information, and source evidence snapshots. ExplainX presence alone is discovery evidence, not sufficient evidence for a trusted rank.

### source_record_entity_links and external_identity_reviews

The effective link state in `source_record_entity_links` is authoritative for scoring eligibility. Append-only `external_identity_reviews` remains the audit history.

Registry contribution requires all of the following:

- the source is ExplainX;
- the ExplainX record is active;
- the canonical entity is active;
- relationship is `same_entity`;
- effective review state is `approved`.

Machine match confidence must not substitute for approval.

### Calibration fixture

`docs/design/fixtures/cross-source-calibration-fixture.json` is the deterministic test oracle for:

- factor normalization and formulas;
- label assignment;
- Global and Indonesia ranking order;
- exclusion and abstention behavior;
- numeric tolerance.

## 3. New Additive Schema Proposal

Add one derived table during IMP-07 implementation:

```sql
CREATE TABLE IF NOT EXISTS cross_source_entity_scores (
    id UUID PRIMARY KEY DEFAULT uuid(),
    score_version TEXT NOT NULL,
    week_start DATE NOT NULL,
    week_end DATE NOT NULL,
    entity_id UUID NOT NULL,
    canonical_name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    region TEXT NOT NULL,
    mention_count BIGINT NOT NULL DEFAULT 0,
    approved_registry_record_count BIGINT NOT NULL DEFAULT 0,
    conversation_source_count BIGINT NOT NULL DEFAULT 0,
    mention_count_score DOUBLE NOT NULL,
    sentiment_score DOUBLE NOT NULL,
    cost_signal_score DOUBLE NOT NULL,
    region_signal_score DOUBLE NOT NULL,
    conversation_score DOUBLE NOT NULL,
    registry_score DOUBLE NOT NULL,
    source_diversity_score DOUBLE NOT NULL,
    review_confidence_score DOUBLE NOT NULL,
    recency_score DOUBLE NOT NULL,
    cost_adjustment DOUBLE NOT NULL,
    sentiment_adjustment DOUBLE NOT NULL,
    cross_source_score DOUBLE NOT NULL,
    ranking_label TEXT NOT NULL,
    factor_breakdown_json TEXT NOT NULL,
    source_evidence_json TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (score_version, week_start, entity_id, region)
);
```

Implementation constraints:

- This table is additive only.
- Do not modify or replace `weekly_entity_metrics`.
- Do not modify or replace `weekly_agent_metrics`.
- Do not add a destructive migration.
- Validate `region`, `ranking_label`, canonical identity, and score bounds in Rust at the service boundary.
- Keep score components and `cross_source_score` in the inclusive `0..100` range.
- Compute using full precision and round only for comparison tolerance or display.
- Use a transaction for a score-version rebuild. Delete only rows for the selected score version and affected week scope, then insert the replacement rows. Roll back the whole rebuild on failure.
- Re-running the same inputs and score version must produce the same unique rows without duplicates.
- A changed formula or normalization contract requires a new `score_version`; it must not silently rewrite the meaning of an older version.
- `factor_breakdown_json` records component inputs, normalized factors, weights, and formula version.
- `source_evidence_json` records non-secret evidence IDs, source names, counts, review state, and freshness metadata. It must not contain access tokens or unnecessary raw personal content.

`cost_adjustment` and `sentiment_adjustment` are explanation deltas from their neutral baselines. The authoritative weighted inputs remain `cost_signal_score` and `sentiment_score`, which are persisted explicitly to reproduce the calibration oracle.

Only `trusted_ranking` rows are persisted in this table during IMP-07. `watchlist`, `needs_review`, and `excluded_from_score` are computed diagnostic results returned by the command, but they do not create score rows. This preserves the fixture requirement that FlowPilot and review-blocked cases have `score_row_created = false`.

## 4. Scoring Rules

Use the approved `cross-source-v1-proposal` oracle.

### Normalization

- Normalize mention volume within the same `week_start` and region cohort.
- Keep Indonesia and Global independent.
- Do not fold `unknown` into either trusted regional ranking.
- Derive sentiment and cost factors from the existing canonical weekly count fields.
- `not_mentioned` is neutral cost evidence, not a positive signal.
- Count trusted source surfaces from Threads, Apify, and approved ExplainX evidence.
- Cap registry presence at the approved contribution. Multiple approved ExplainX records may increase evidence count but must not multiply the registry factor beyond `100`.
- For the one-week calibration fixture, `recency_score` is `100` for current trusted rows. Production decay remains outside this prototype unless separately approved under a new calibrated contract.

### Formula

```text
conversation_score =
    mention_count_score * 0.55
  + sentiment_score     * 0.25
  + cost_signal_score   * 0.15
  + recency_score       * 0.05

cross_source_score =
    conversation_score       * 0.55
  + registry_score           * 0.20
  + source_diversity_score   * 0.10
  + review_confidence_score  * 0.10
  + recency_score            * 0.05
```

Clamp each factor and final score to `0..100`.

### Required oracle result

- Global: Claude Code ranks above Ponytail, which ranks above Caveman.
- Indonesia: Claude Code is the only trusted row.
- FlowPilot is `watchlist`, not trusted ranking.
- NovaForge, Codex, and UnknownNewTool are `needs_review` and receive no score row.
- Generic MCP editorial evidence is `excluded_from_score`.
- Registry-only evidence cannot create a trusted ranking.

## 5. Ranking Labels

### trusted_ranking

The entity is active, canonical identity is resolved, and a current `weekly_entity_metrics` conversation row exists. Approved ExplainX evidence may contribute, but is not required.

### watchlist

An active canonical entity has approved ExplainX `same_entity` evidence but no current regional conversation row. It is returned for discovery visibility without a score row or rank.

### needs_review

Evidence exists, but identity is pending, ambiguous, missing, or otherwise unresolved. It receives no registry boost, score row, or trusted rank.

### excluded_from_score

Evidence is rejected, generic, no-product, inactive, unsupported relationship evidence, or otherwise ineligible. It receives no score row or trusted rank.

Labels are mutually exclusive per evaluated evidence case. Every non-trusted result must include an explainable reason.

## 6. Source Trust Rules

1. A resolved, active canonical entity and current regional conversation row are required for `trusted_ranking`.
2. Only an active, approved ExplainX `same_entity` link contributes `registry_score`.
3. Pending links do not contribute registry or review-confidence factors.
4. Rejected links are ignored for score contribution and surfaced only as excluded diagnostics where relevant.
5. Ambiguous links remain excluded until a later explicit review resolves them.
6. Approved `child_resource`, `related_entity`, and `mentioned_entity` links do not contribute to IMP-07 scoring.
7. Registry-only entities may be `watchlist`, but cannot become trusted ranking without conversation evidence.
8. Candidate Review and External Identity Review remain independent. An approved social candidate still requires canonical linkage and canonical weekly evidence.
9. Match confidence, name similarity, and fuzzy matching never grant score eligibility.
10. Failed or partial source collection does not prove absence and must not silently reduce a score.

## 7. Service and Command Plan

### Service

Add `src-tauri/src/services/cross_source_score_aggregator.rs` during implementation.

Suggested responsibilities:

1. Load the selected score version and eligible weekly cohorts.
2. Load active canonical metadata and effective approved ExplainX links.
3. Classify all evidence cases into the four ranking labels.
4. Normalize factors independently by week and region.
5. Compute deterministic factors and final scores.
6. Build factor and source-evidence JSON using typed serializable models.
7. Transactionally replace only the selected version/week score rows.
8. Load ranked previews ordered by `cross_source_score DESC`, then deterministic tie-breakers such as `mention_count DESC` and `canonical_name ASC`.
9. Return non-ranked diagnostics without persisting false score rows.

Keep score weights as a versioned Rust constant for this prototype. Do not activate unrelated placeholders in `config/scoring.yml` until config parsing and validation are explicitly scoped.

### Command

Register:

```text
aggregate_cross_source_entity_scores()
```

The command response should include:

- score version and covered week;
- scored rows;
- trusted ranking rows;
- watchlist rows;
- needs-review rows;
- excluded rows;
- Top Global preview;
- Top Indonesia preview;
- diagnostic reasons grouped by label;
- fixture validation result when invoked by the isolated fixture test path, not by production data aggregation.

The production command must not read the fixture or report a fixture pass based on live/local data. Fixture validation belongs to a deterministic service test or a dedicated validation helper.

## 8. Read-only UI Plan

Add one compact `Cross-source Score Preview` section after canonical weekly metrics.

Show:

- entity;
- region;
- cross-source score;
- ranking label;
- key factor values;
- registry evidence count/state;
- concise explanation.

Provide separate Top Indonesia and Top Global tables. Show diagnostic counts for watchlist, needs review, and excluded cases without mixing them into ranking tables. The panel is read-only except for the explicit aggregate/refresh action. Do not redesign the guided workflow, alter review actions, or change report export.

## 9. Test Plan

### Calibration fixture

- Parse the checked-in fixture with typed structures.
- Recompute every trusted factor within the fixture numeric tolerance.
- Assert Global order is Claude Code, Ponytail, Caveman.
- Assert Indonesia contains only Claude Code.
- Assert exact labels and score-row behavior for all eight fixture entities.

### Trust and eligibility

- FlowPilot is `watchlist` and creates no score row.
- NovaForge, Codex, and UnknownNewTool are `needs_review` or excluded as specified and create no score rows.
- Generic MCP editorial evidence is excluded.
- A registry-only entity never enters trusted ranking.
- A pending ExplainX link contributes no registry boost.
- An approved `same_entity` link can contribute registry and review-confidence scores.
- Approved child, related, and mentioned relationships contribute no score.
- Rejected and ambiguous links remain ineligible.

### Persistence and compatibility

- Aggregation leaves `weekly_entity_metrics` byte-for-byte logically unchanged.
- Aggregation leaves `weekly_agent_metrics` unchanged.
- Re-running the same version and week creates no duplicate rows.
- A failed rebuild rolls back to the prior score rows.
- Existing MVP full-flow and raw-insert regression tests continue to pass.
- New DB-backed tests use unique DuckDB paths and make no process-global `DATABASE_PATH` mutation.
- Default parallel `cargo test` remains reliable; serial tests remain a secondary compatibility check.
- No test performs a live Threads, Apify, or ExplainX call.

## 10. Explicitly Not Implemented

IMP-07 does not include:

- momentum, week-over-week growth, velocity, rank change, or momentum labels;
- Programming Fit;
- LLM scoring or classification;
- live ExplainX scraping;
- fuzzy merge or automatic identity approval;
- scoring child resources, related entities, or mentioned entities;
- replacing or mutating `weekly_entity_metrics` or `weekly_agent_metrics`;
- changing current report export behavior;
- changing collectors, classifiers, Candidate Review, or External Identity Review;
- a blended Indonesia/Global score;
- production recency decay beyond the approved one-week fixture contract.

## 11. Risks and Open Questions

- The fixture locks the prototype formula. Weight or normalization changes require a new score version and updated oracle.
- Sparse Indonesia cohorts can overstate relative strength. Always display evidence counts and preserve independent regional normalization.
- Registry boost can dominate weak conversation data if uncapped. Require conversation evidence and cap registry contribution.
- Multiple approved registry records may overcount one product. Preserve counts for audit but cap the factor.
- Production recency decay is not calibrated. Do not invent it inside IMP-07.
- A derived JSON explanation can drift from numeric columns. Generate both from the same typed factor object.
- DuckDB rebuild failures could leave partial rows without a transaction. Treat transactional replacement and rollback tests as release gates.
- Non-ranked diagnostics can be mistaken for scored rows. Keep command/UI models explicit and tables visually separate.
- Small denominators can make sentiment or cost factors look overconfident. Display raw evidence counts with factors.

## 12. Safe Implementation Order

1. Add the additive table initialization and schema documentation.
2. Add typed score, factor, evidence, summary, and diagnostic models.
3. Implement pure factor calculations and fixture parser tests before persistence.
4. Implement the aggregator service with explicit eligibility gates.
5. Add transactional idempotent persistence and rollback tests.
6. Register `aggregate_cross_source_entity_scores()` and its response model.
7. Add fixture-based ranking and trust-rule tests.
8. Add the read-only UI preview without changing existing workflow semantics.
9. Update project, schema, progress, and handoff documentation.
10. Run frontend build, Rust format/check, parallel and serial tests, diff check, and security scan.
11. Commit as a focused IMP-07 checkpoint only after every acceptance gate passes.

No coding step should begin until this brief is explicitly accepted as the implementation boundary.
