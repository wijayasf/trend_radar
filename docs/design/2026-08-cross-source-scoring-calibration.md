# Cross-source Scoring Calibration

Date: 2026-08-21
Status: Deterministic fixture proposal, not implemented
Design dependency: `docs/design/2026-08-cross-source-scoring-momentum.md`
Fixture: `docs/design/fixtures/cross-source-calibration-fixture.json`

## 1. Purpose

This calibration defines a small, deterministic oracle for the future IMP-07 Cross-source Score Prototype. It translates the design principles into expected eligibility, labels, factor values, and regional ranking order before any schema or scoring code is written.

The fixture is synthetic. It contains no live Threads, Apify, or ExplainX data and no credentials. Its expected numeric values calibrate ordering and explainability for one fixed week; they are not production claims about the named tools.

## 2. Calibration Labels

- `trusted_ranking`: a resolved canonical entity has current regional conversation evidence. Only approved `same_entity` external links contribute registry factors.
- `watchlist`: trusted registry evidence exists, but there is no current regional conversation row. The entity is visible for discovery but has no trusted rank.
- `needs_review`: conversation or registry evidence exists, but identity is pending, ambiguous, or missing.
- `excluded_from_score`: evidence is rejected, generic, no-product, inactive, or otherwise ineligible.

These labels are mutually exclusive in the fixture. A later UI may group `watchlist` and `needs_review` together, but it must preserve the reason.

## 3. Reference Factor Rules

All numeric factors use `0..100` and round to two decimal places. The fixture defines the following reference behavior for deterministic IMP-07 tests.

### Mention Count

Normalize only within the same week and region, using trusted-ranking candidates:

```text
mention_count_score =
  100 * ln(1 + entity_mentions) / ln(1 + maximum_eligible_mentions_in_region)
```

`region_signal_score` equals this region-local normalized value for display only. It is not separately weighted in `cross_source_score`.

### Sentiment

```text
sentiment_score = clamp(
  50
  + 50 * (positive_count - negative_count) / mention_count
  - 15 * mixed_count / mention_count,
  0,
  100
)
```

Positive sentiment helps, but its conversation weight remains below mention volume. Neutral evidence retains the baseline. Mixed and negative evidence reduce the factor.

### Cost Signal

```text
cost_signal_score = clamp(
  50
  + 50 * (cost_positive_count - cost_negative_boros_count) / mention_count
  - 20 * cost_mixed_count / mention_count,
  0,
  100
)
```

`not_mentioned` is neutral, not positive. Cost concern reduces the factor and remains visible in the raw counts.

### Registry Presence and Review Confidence

- `registry_presence_score = 100` only for an active, current ExplainX record with an approved `same_entity` link; otherwise `0`.
- `review_confidence_score = 100` only when that registry contribution is explicitly approved; otherwise `0`.
- Pending and ambiguous links never contribute a partial registry or review score.
- Approved child-resource, related-entity, and mentioned-entity links are not eligible in IMP-07.
- A pending registry candidate does not invalidate a separately resolved conversation entity; it is simply omitted from the score factors.

### Source Diversity

Count distinct trusted surfaces from the fixed set `threads`, `apify`, and `explainx`:

```text
source_diversity_score = 100 * trusted_surface_count / 3
```

ExplainX counts only after approved same-entity review. A pending ExplainX link does not increase diversity.

### Recency

Every trusted conversation row in this one-week fixture is current, so `recency_score = 100`. Production decay remains an IMP-07 calibration question; this fixture only verifies that stale or absent rows cannot gain current-rank credit.

### Combined Scores

The fixture applies the proposal from DESIGN-01:

```text
conversation_score =
    mention_count_score * 0.55
  + sentiment_score     * 0.25
  + cost_signal_score   * 0.15
  + recency_score       * 0.05

cross_source_score =
    conversation_score       * 0.55
  + registry_presence_score  * 0.20
  + source_diversity_score   * 0.10
  + review_confidence_score  * 0.10
  + recency_score            * 0.05
```

Numeric assertions may use a tolerance of `0.02` after two-decimal rounding.

## 4. Fixture Entities

### Claude Code

- Resolved canonical `agent_tool`.
- Strong Global conversation and the only Indonesia conversation row.
- Active ExplainX record with an approved `same_entity` link.
- Receives registry, review, and three-surface diversity support.
- Expected Global rank 1 and Indonesia rank 1.

### Ponytail

- Resolved canonical `skill_mode` with substantial Global conversation.
- ExplainX link remains pending and contributes no registry or review score.
- Ranks from trusted Threads/Apify conversation only.
- Expected Global rank 2 and no Indonesia row.

### Caveman

- Resolved canonical `skill_mode` with moderate Global conversation.
- No ExplainX trust contribution.
- Lower mention volume and more cost concern keep it below Ponytail.
- Expected Global rank 3 and no Indonesia row.

### FlowPilot

- Resolved canonical entity with an approved ExplainX same-entity record.
- Has no current conversation row.
- Expected label is `watchlist`; no trusted Global or Indonesia rank is created.

### NovaForge

- ExplainX candidate with a pending identity link and no resolved canonical identity.
- Expected label is `needs_review`; it cannot receive registry boost or a trusted rank.

### Codex

- Has conversation evidence, but the alias resolves to multiple possible identities.
- Expected label is `needs_review`; no fuzzy merge and no trusted score.

### UnknownNewTool

- Has weak conversation evidence but no canonical alias or resolved identity.
- Expected label is `needs_review`; it remains in candidate review/watchlist surfaces only.

### MCP Weekly Roundup

- A generic/editorial no-product record with a rejected relationship.
- Expected label is `excluded_from_score` even though generic conversation evidence exists.
- This verifies that standalone MCP/editorial text cannot enter canonical ranking.

## 5. Expected Ranking Scenarios

### Scenario A - Global Trusted Ranking

Expected order and rounded prototype scores:

| Rank | Entity | Conversation | Cross-source | Why |
| --- | --- | ---: | ---: | --- |
| 1 | Claude Code | 85.84 | 92.21 | Strong conversation plus approved registry and three trusted surfaces. |
| 2 | Ponytail | 77.04 | 54.04 | Conversation-only rank; pending ExplainX evidence adds nothing. |
| 3 | Caveman | 63.89 | 43.47 | Moderate one-source conversation with a cost-concern penalty. |

FlowPilot is watchlist-only because it has no conversation row. NovaForge, Codex, UnknownNewTool, and MCP Weekly Roundup are excluded from trusted ranking for the explicit reasons stored in the fixture.

### Scenario B - Indonesia Trusted Ranking

Claude Code is the only eligible Indonesia row and ranks first with expected conversation score `86.75` and cross-source score `92.71`. Global mention volume does not participate in Indonesia normalization. Ponytail and Caveman do not appear because they have no Indonesia conversation row.

### Scenario C - Registry-only Evidence

FlowPilot demonstrates that approved ExplainX presence can create a trusted discovery/watchlist signal but cannot create a ranked score without `weekly_entity_metrics` conversation evidence. NovaForge demonstrates that a pending registry candidate is not trusted even for registry boost.

### Scenario D - Ambiguous or Missing Identity

Codex remains `needs_review` despite seven raw conversation mentions because identity is ambiguous. UnknownNewTool remains `needs_review` because no canonical alias exists. Neither receives a score row, and no fuzzy or first-match merge is permitted.

## 6. Required Assertions for IMP-07 Tests

1. Exactly three Global rows have label `trusted_ranking` in the base fixture.
2. Global order is Claude Code, Ponytail, Caveman.
3. Exactly one Indonesia row is trusted: Claude Code.
4. Claude Code receives ExplainX registry and review factors in both regional rows.
5. Ponytail's pending ExplainX link contributes zero registry and review factors.
6. Caveman ranks below Ponytail because its normalized conversation evidence is weaker.
7. FlowPilot receives no ranked score row and appears as `watchlist`.
8. NovaForge, Codex, and UnknownNewTool receive no score row and appear as `needs_review`.
9. MCP Weekly Roundup is `excluded_from_score` and never appears in canonical ranking.
10. Every trusted row exposes raw inputs, factor values, score version, and exclusion-free explanation.
11. Reprocessing the same fixture is idempotent.
12. Existing `weekly_entity_metrics` rows and values remain unchanged.

## 7. IMP-07 Acceptance Criteria

Minimum acceptance:

1. Create an additive `cross_source_entity_scores` table.
2. Use only resolved, active canonical identities.
3. Use only approved external `same_entity` links for registry contribution.
4. Exclude ambiguous, missing, pending, rejected, and ineligible relationship evidence from trusted scoring.
5. Keep Indonesia and Global ranking cohorts separate.
6. Persist and return a factor breakdown for every score row.
7. Do not modify or replace `weekly_entity_metrics`.
8. Do not implement momentum, WoW, velocity, or rank-change logic.
9. Do not implement Programming Fit, LLM scoring, live ExplainX scraping, fuzzy merge, or automatic review approval.
10. Pass deterministic fixture assertions, including exact labels, ranking order, and expected numeric scores within tolerance.

## 8. Calibration Decisions Still Open

- Production recency decay and stale-data thresholds.
- Minimum conversation count for datasets larger than this fixture.
- Whether production mention normalization uses log scaling, cohort percentile, or a hybrid after real-data evaluation.
- Whether multiple approved ExplainX records remain capped at one registry contribution.
- Whether `review_confidence_score` remains binary or becomes a provenance-based value without using machine match confidence as approval.

These questions do not block fixture-based IMP-07 prototyping, but any changed rule requires a new `score_version` and updated expected fixture output.
