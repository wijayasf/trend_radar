# Latest Handoff

Date: 2026-08-14
Session: 041-budget-safe-apify-replay
Agent: Codex

## Current State

Apify discovery is budget-safe by default. A crawl sends all seeds in one actor request, successful live datasets are cached locally, and replay applies the latest filtering pipeline without network usage. No live Apify crawl was run during this patch.

## Key Changes

- Confirmed and tested one actor input with the complete `keywords` array.
- Added ignored cache path `data/cache/apify-last-run.json` and command `replay_last_apify_crawl`.
- Added `Reprocess Last Apify Result` UI action and explicit no-usage copy.
- Added default-off `APIFY_LIVE_CRAWL_ENABLED`, per-process live-run limit, cache TTL note, and existing bounded timeout configuration.
- Marked Apify single-seed testing as a live, credit-consuming debug action subject to the same guard.
- Rejected `Agent Engineer`, `AI Agent Engineer`, `Copilots`, and `YouTube` as unknown candidates while preserving configured `GitHub Copilot` detection.
- Required launch/identity language plus product context for domain-shaped candidates. `folk.com` can remain pending from launch evidence and stays out of weekly metrics until approved.

## Validation Snapshot

- Cache replay fixture: 5 cached posts, 2 included (`Claude Code`, launch-evidenced `folk.com`), 3 noise posts filtered.
- `folk.com` was pending and excluded from weekly metrics before approval; it entered metrics only after explicit test approval.
- Frontend build, Rust format/check, full sample flow, raw insert regression, entity, Apify, cost, weekly, candidate, and reset suites passed.
- Existing Rust placeholder/dead-code warnings remain unchanged.
- Security scan found only literal pattern names in historical reports, with no secret values. Cache/runtime/generated paths remain ignored and untracked.

## Pending

- Run a manual UI replay against a real cache when one is available locally.
- Do not run another live Apify crawl until budget usage is intentionally approved.
- Push only when explicitly requested after reviewing the local commit set.

## Risk Note

The cache is local and ignored; machines without a prior successful live run will receive a friendly missing-cache error. The live-run counter resets when the desktop process restarts, so it is a session guard rather than a durable billing quota.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
