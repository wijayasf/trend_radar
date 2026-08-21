# Cross-source Scoring and Momentum Design

Date: 2026-08-21
Status: Proposal, not implemented
Target follow-up: IMP-07 Cross-source Score Prototype

## 1. Background

Trend Radar now has the identity and observation foundation needed to discuss multi-source scoring safely:

- `canonical_entities` provides source-independent UUID identities.
- `entity_aliases` provides scoped, provenance-aware aliases without assuming normalized-name uniqueness.
- `agent_mentions` can link social mentions to canonical entities through explicit, conservative identity resolution.
- `weekly_entity_metrics` aggregates resolved social mentions by canonical entity, week, and region while preserving the existing `weekly_agent_metrics` pipeline.
- ExplainX local import persists registry records, source observations, and pending source-to-entity links without making live requests.
- External Identity Review provides explicit Approve, Reject, and Mark Ambiguous decisions with append-only history.
- GitHub Actions validates the frontend, Rust code, deterministic tests, and tracked-secret safety.

PR #1 and PR #2 established these capabilities additively. No cross-source score, momentum calculation, or Programming Fit model exists today.

## 2. Problem Statement

`weekly_entity_metrics` ranks canonical entities from conversation evidence, but it does not combine that evidence with reviewed ecosystem records. ExplainX can establish that a named tool is listed and provide useful metadata, yet listing presence alone is not proof of adoption, usage, or trend.

Trend Radar therefore needs an explainable model that answers two separate questions before combining them:

1. What does public conversation say about a canonical entity in a region and week?
2. What trusted registry evidence exists for that same canonical entity?

The combined score must preserve source meaning, identity review boundaries, regional views, and an auditable factor breakdown. It must not turn an uncertain identity or a registry listing into an artificial trend.

## 3. Source Roles

### Threads and Apify

Threads and the Apify fallback represent conversation evidence:

- mention volume and social visibility;
- sentiment about the named entity;
- cost, quota, pricing, and token-usage concerns;
- recency of discussion;
- regional signal split into Indonesia, Global, and Unknown;
- distinct observed conversation sources where available.

These sources do not provide actual product telemetry. Their metrics describe observed public discussion only.

### ExplainX

ExplainX represents registry and discovery evidence:

- evidence that a tool or related resource is listed;
- category, tags, description, URLs, platform, and pricing metadata;
- source observations and first/last-seen history;
- possible relationships to canonical entities.

ExplainX presence does not by itself prove adoption, momentum, or positive quality. Only an explicitly approved `same_entity` link may contribute to the trusted cross-source prototype. Approved child resources and related or mentioned entities remain distinct evidence and are excluded from IMP-07 scoring.

### Canonical Identity and Human Review

Canonical UUIDs are the join boundary. A normalized name, exact text match, or high machine confidence is insufficient on its own. Social mentions must already be resolved to an active canonical entity, and ExplainX records must have an explicitly approved `same_entity` link before the two surfaces can contribute to one score.

## 4. Design Principles

1. Human-reviewed identity precedes cross-source trust.
2. No fuzzy match or high-confidence heuristic may auto-merge identities.
3. Pending, ambiguous, rejected, missing-alias, and unresolved links do not affect trusted cross-source scores.
4. Conversation and registry dimensions remain visible separately before and after combination.
5. Every score stores its version, inputs, factor values, and exclusion reasons.
6. Unknown candidates stay outside canonical ranking until Candidate Review and canonical linkage complete.
7. A registry-only entity must not outrank a discussed entity merely because it is listed.
8. Existing `weekly_agent_metrics` and `weekly_entity_metrics` remain unchanged.
9. Missing observations mean not observed, not confirmed absent.
10. Regional ranking is computed within each region so global volume cannot drown out Indonesia signal.

## 5. Proposed Scoring Dimensions

All prototype dimensions should be normalized to `0..100` and retained individually. Exact normalization constants require fixture-based calibration before implementation.

### mention_count_score

A bounded, log-scaled or cohort-percentile representation of `weekly_entity_metrics.mention_count`. Log scaling prevents one high-volume entity from dominating a small local dataset. Normalization occurs within the same week and region.

### sentiment_score

A conversation-quality dimension derived from positive, neutral, negative, and mixed counts. Neutral is the baseline; positive evidence raises the value, while negative and mixed evidence reduce it by documented amounts. The raw counts remain visible so a small denominator is not presented as high confidence.

### cost_signal_score

A cost-perception dimension derived from cost-positive, cost-negative/boros, cost-mixed, and not-mentioned counts. `not_mentioned` is neutral evidence, not a positive signal. Cost-positive evidence raises the value; negative/boros evidence lowers it; mixed evidence applies a smaller penalty.

### region_signal_score

A descriptive regional-strength dimension based on an entity's evidence within one region. It must not reward Global solely for having more total posts. In the prototype this factor should be displayed for explanation but not added as a separate top-level weight, because mention normalization already occurs per region and a second regional weight would double count volume.

### registry_presence_score

A bounded ExplainX dimension available only when an active ExplainX record has an approved `same_entity` link. Presence is the baseline; current status, observation freshness, and metadata completeness may refine it. It must remain zero for pending, rejected, ambiguous, child-resource, related-entity, or mentioned-entity links.

### source_diversity_score

A bounded count of distinct trusted evidence surfaces contributing to the row. Conversation source types can include Threads and Apify. ExplainX counts only through an approved `same_entity` link. This is a modest corroboration bonus, not a substitute for conversation volume.

### review_confidence_score

An audit dimension describing whether every cross-source join passed the required human review gate. It must not use `match_confidence` as an approval proxy. For IMP-07, an explicit approved same-entity review is trusted; all other external states are ineligible rather than softly penalized.

### recency_score

A decay based on the most recent eligible conversation and registry observation timestamps relative to the score's `week_end` or computation time. Partial or failed collection runs do not prove staleness. The decay window and clock boundary must be recorded in the score version.

### momentum_score

A future dimension comparing consecutive canonical week buckets. It remains null and unimplemented in IMP-07. It should not be inferred from a single weekly row or from ExplainX listing presence.

## 6. Proposed Formula Draft

The following is a calibration proposal, not implemented behavior:

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

Guardrails:

- A current `weekly_entity_metrics` row is required for a trusted ranked result in IMP-07. Registry-only entities may appear in a separate discovery queue but receive no ranked cross-source row.
- Every component is clamped to `0..100`; the final score is also clamped to that range.
- Negative sentiment and cost/boros concern reduce their respective dimensions.
- Ambiguous, pending, rejected, missing-review, and stale unresolved identity evidence is excluded, not disguised as a small positive value.
- Stale eligible data reduces `recency_score`; failed or partial collection runs are recorded as coverage limitations.
- The score stores `score_version` and factor values so later calibration does not rewrite historical meaning silently.

`config/scoring.yml` currently contains unused future-oriented ranking placeholders, including velocity. IMP-07 must not silently treat those values as an active contract. The prototype should define a separate explicit score version and only move calibrated weights into configuration once parsing and validation are part of the scoped implementation.

## 7. Momentum and Week-over-Week Design

Momentum is a later phase built from consecutive `(entity_id, region, week_start)` rows after cross-source score semantics stabilize.

Proposed metrics:

- `week_over_week_mention_growth`: change in mention count versus the previous comparable week;
- `rank_change`: prior regional rank minus current regional rank, so positive means improvement;
- `new_entity_this_week`: current eligible signal exists and no prior eligible week exists;
- `reappearing_entity`: current signal exists after one or more missing weeks;
- `velocity_score`: bounded combination of mention delta, rank change, source expansion, and recency;
- `momentum_label`: `rising`, `stable`, `cooling`, `new_signal`, or `needs_review`.

Proposed label semantics:

- `new_signal`: first trusted weekly signal, not infinite growth;
- `rising`: meaningful positive change above a calibrated minimum baseline;
- `cooling`: meaningful negative change below a calibrated threshold;
- `stable`: movement remains within the neutral band;
- `needs_review`: evidence exists but identity eligibility is unresolved; shown outside trusted rankings.

Small denominators require smoothing. A change from one mention to two should not be treated the same as a change from 100 to 200 without minimum-volume and confidence indicators. Missing weeks must distinguish no observation from a completed zero-result collection run.

## 8. Indonesia and Global Handling

- Indonesia and Global remain separate rankings and normalization cohorts.
- A canonical entity may be rising globally while absent, stable, or cooling in Indonesia.
- Indonesia scores are calibrated against Indonesia evidence, not against global volume.
- Unknown-region rows remain diagnostic and must not be folded into either regional score automatically.
- A future comparative view may show both regional scores side by side, but it should not produce one blended worldwide score by default.
- Sparse Indonesia rows should display evidence counts and confidence rather than receive a volume multiplier intended for global data.

## 9. Human Review Rules

### External Identity Review

- `approved` plus `same_entity`: eligible for ExplainX registry contribution.
- `approved` plus `child_resource`, `related_entity`, or `mentioned_entity`: excluded from IMP-07 score; retained as relationship evidence for later design.
- `pending`: excluded from trusted cross-source scoring.
- `rejected`: ignored for scoring.
- `ambiguous`: excluded until a later explicit decision resolves it.
- Machine `match_confidence` remains diagnostic and never grants eligibility.

### Candidate Review

Candidate Review continues to answer whether a discovered social candidate is a named canonical-worthy entity. External Identity Review answers how an external source record relates to an existing canonical entity. These workflows remain separate. A pending or ignored social candidate cannot enter `weekly_entity_metrics`, and an approved candidate still requires canonical identity linkage before cross-source scoring.

## 10. Recommended IMP-07 Scope

### IMP-07 - Cross-source Score Prototype

Implement the smallest safe, read-derived layer:

1. Add an additive `cross_source_entity_scores` table keyed by week, canonical entity, region, and score version.
2. Read existing `weekly_entity_metrics` as conversation input without modifying it.
3. Join only active ExplainX records connected through explicitly approved `same_entity` links.
4. Persist the component scores, final score, input counts, `score_version`, explanation JSON, and computation timestamps.
5. Rebuild prototype rows idempotently and transactionally.
6. Return explicit counts for eligible rows and exclusions by pending, rejected, ambiguous, relationship type, missing conversation evidence, and stale/invalid source data.
7. Add a read-only UI preview with separate Top Indonesia and Top Global tables plus factor breakdown.
8. Add isolated tests for identity eligibility, region separation, registry-only exclusion, idempotency, and legacy metrics compatibility.

Suggested minimum persisted fields:

```text
id
week_start
week_end
entity_id
canonical_name
entity_type
region
mention_count
approved_registry_record_count
conversation_source_count
mention_count_score
sentiment_score
cost_signal_score
registry_presence_score
source_diversity_score
review_confidence_score
recency_score
cross_source_score
score_version
explanation_json
computed_at
```

Explicitly not in IMP-07:

- WoW, velocity, rank change, or momentum labels;
- Programming Fit;
- LLM-based scoring or classification;
- live ExplainX scraping;
- fuzzy matching or automatic identity approval;
- scoring approved child/related/mentioned resources;
- replacement or mutation of current weekly tables and reports;
- automatic report export changes.

## 11. Risks and Open Questions

1. How much weight should one reviewed ExplainX listing receive after fixture calibration?
2. Should registry freshness decay from `last_seen_at`, the latest observation, or both?
3. What minimum conversation evidence is required before a row appears in trusted ranking?
4. Which normalization is most stable for sparse Indonesia cohorts: log cap, percentile, or both?
5. How strongly should negative cost/boros evidence reduce a score when sample size is small?
6. Should multiple approved ExplainX records for one entity add evidence, or remain capped at one registry-presence contribution?
7. How should partial and failed collection runs affect coverage confidence without implying absence?
8. When should an approved social candidate become eligible if its canonical linkage has not run yet?
9. How much human review is required before related and child-resource evidence can support a future non-identity dimension?
10. What score-version migration policy preserves historical auditability after weights change?

Before IMP-07 implementation, validate the formula against a small fixture containing discussed-and-listed, discussed-only, listed-only, ambiguous, rejected, and sparse-Indonesia examples. The expected ranking order and full factor breakdown should be agreed before schema or service code is added.
