# Latest Handoff

Date: 2026-07-13
Session: 027-candidate-review-workflow
Agent: Codex

## Current State

Candidate Review Workflow MVP has been added.

Unknown candidates detected by the entity detector can now be reviewed from the UI and backend commands:

1. list candidates
2. approve candidate as canonical entity/category
3. ignore false positive candidate
4. reset candidate back to pending

## Commands Added

- `list_candidate_entities`
- `approve_candidate_entity(candidate_name, reviewed_as, reviewed_category, note?)`
- `ignore_candidate_entity(candidate_name, note?)`
- `reset_candidate_review(candidate_name)`

## Data Model

Review state is stored directly on `agent_mentions`:

- `review_status`: `pending`, `approved`, or `ignored`
- `reviewed_as`
- `reviewed_category`
- `review_note`
- `reviewed_at`

Known aliases default to `approved`. New unknown candidates default to `pending`.

## Aggregation Behavior

- Known aliases are included in weekly metrics.
- Approved candidates are included in weekly metrics under their reviewed canonical name/category.
- Ignored candidates are excluded.
- Pending `unknown_candidate` rows are excluded from Top metrics.

## UI

New section: `Candidate Review`

It shows:

- pending/approved/ignored counts
- candidate name
- mention count
- sample snippet
- canonical name input
- category select
- review note input
- Approve, Ignore, and Reset buttons

## Report Export

Markdown weekly report now includes a lightweight `Candidate Review Notes` section.

## Validation

- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check`: passed.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.
- `data/sample_threads_posts.json` parse check: passed.
- `src-tauri/data`: empty.

## Sample Validation Result

- `NovaForge` pending candidate detected.
- `FlowPilot` pending candidate detected.
- `NovaForge` approved as `coding_agent`, updated `1` mention, and appeared in weekly metrics.
- `FlowPilot` ignored, updated `1` mention, and did not appear in weekly metrics.

## Pending / Recommended Next Step

Run manual UI smoke test for the Candidate Review panel:

1. Run Discovery Crawl or Import Sample Posts
2. Detect Agent Mentions
3. Refresh Candidate Review
4. Approve `NovaForge`
5. Ignore `FlowPilot`
6. Aggregate Weekly Metrics
7. Confirm approved candidate appears and ignored candidate stays out

Then consider durable `entity_review_decisions` or alias-config promotion if reviewed candidates should apply automatically to future posts.

## Risk Note

- Current review state is row-level on `agent_mentions`, not a global reusable candidate decision registry.
- Existing Rust placeholder dead-code warnings remain.
- Token and `.env` contents were not read or printed.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
