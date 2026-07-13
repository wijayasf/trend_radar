# Progress Report: Session 027 - Candidate Review Workflow MVP

Date: 2026-07-13
Agent: Codex

## Objective

Add a minimal Candidate Review workflow for `unknown_candidate` entity mentions so reviewers can approve, ignore, or normalize newly discovered tool/entity names before weekly aggregation.

## Completed

- Added review fields directly to `agent_mentions`:
  - `review_status`
  - `reviewed_as`
  - `reviewed_category`
  - `review_note`
  - `reviewed_at`
- Added candidate review commands:
  - `list_candidate_entities`
  - `approve_candidate_entity`
  - `ignore_candidate_entity`
  - `reset_candidate_review`
- Added backend service `candidate_review`.
- Added Candidate Review UI section:
  - pending/approved/ignored counts
  - candidate name and mention count
  - sample snippet
  - canonical name input
  - category select
  - review note input
  - Approve, Ignore, and Reset buttons
- Updated weekly aggregation to exclude ignored candidates and pending `unknown_candidate` rows from Top metrics.
- Approved candidates now enter weekly metrics under the reviewed canonical name/category.
- Added light Candidate Review Notes section to Markdown report export.
- Updated mock/sample data with `FlowPilot` so approve and ignore paths are both testable.
- Updated schema docs and README.

## Validation

- `npm run build` passed.
- `cargo fmt --check` passed.
- `cargo check` passed.
  - Existing placeholder dead-code warnings remain.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1` passed.
- `data/sample_threads_posts.json` parsed successfully.
- `src-tauri/data` remained empty.

## Sample Candidate Review Test Result

- `NovaForge` is detected as a pending candidate.
- `FlowPilot` is detected as a pending candidate.
- `NovaForge` approval updated `1` mention and made it eligible for weekly metrics as `coding_agent`.
- `FlowPilot` ignore updated `1` mention and kept it out of weekly metrics.
- Weekly metrics still generated successfully after candidate review.

## Not Completed

- No real UI click smoke was run for the Candidate Review panel.
- No commit or push was performed in this session.
- Candidate decisions are stored on current `agent_mentions` rows only; a future durable decision table may be useful for auto-applying decisions to future posts.

## Risk Note

- Direct row-level review fields are intentionally simple for MVP but do not yet represent a reusable global alias/candidate decision registry.
- Pending candidates are excluded from Top metrics, so reviewers should approve relevant candidates before relying on rankings.
- Token and `.env` contents were not read or printed.

## Next Recommended Task

Run a manual UI smoke test for Candidate Review, then consider a durable `entity_review_decisions` table or alias-config promotion flow for approved candidates.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
