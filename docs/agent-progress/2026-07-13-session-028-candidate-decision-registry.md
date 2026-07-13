# Progress Report: Session 028 - Candidate Decision Registry

Date: 2026-07-13
Agent: Codex

## Objective

Add a durable Candidate Decision Registry so approved or ignored `unknown_candidate` entities are automatically applied to future detections.

## Completed

- Added `entity_review_decisions` DuckDB table.
- Added decision model and list result types.
- Added backend decision listing command:
  - `list_entity_review_decisions`
- Updated existing candidate review commands:
  - approve now inserts/updates `entity_review_decisions`
  - ignore now inserts/updates `entity_review_decisions`
  - reset deletes the durable decision and resets matching mentions to pending
- Updated entity detector to load registry decisions before candidate matching.
- Approved future candidate detections now become:
  - `detection_source = reviewed_candidate`
  - `needs_review = false`
  - `review_status = approved`
  - canonical `agent_name`
  - approved `category`
- Ignored future candidate detections now remain excluded via:
  - `review_status = ignored`
  - `needs_review = false`
- Updated Candidate Review UI:
  - approved decision count
  - ignored decision count
  - Decision Registry table
  - reset button for durable decisions
- Updated targeted full-flow test to validate durable approve/ignore persistence.
- Updated README and DuckDB schema docs.

## Validation

- `npm run build` passed.
- `cargo fmt --check` passed.
- `cargo check` passed.
  - Existing placeholder dead-code warnings remain.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1` passed.

## Sample Persistence Validation

- `NovaForge` is detected as pending.
- `NovaForge` is approved as `coding_agent`.
- A new post containing `NovaForge` is saved.
- Re-running entity detection applies the registry decision immediately:
  - `NovaForge`
  - `coding_agent`
  - `reviewed_candidate`
  - not pending
- `FlowPilot` is ignored.
- A new post containing `FlowPilot` is saved.
- Re-running entity detection keeps `FlowPilot` ignored and out of weekly metrics.

## Not Completed

- No commit or push was performed.
- No manual native UI click smoke test was run.

## Risk Note

- Decision key normalization is intentionally simple and local: lowercase alphanumeric text with normalized spacing.
- Reset deletes the durable decision; future detections become pending again.
- Token and `.env` contents were not read or printed.

## Next Recommended Task

Manual UI smoke test for the Decision Registry table, then commit the registry feature if the UI behavior looks right.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
