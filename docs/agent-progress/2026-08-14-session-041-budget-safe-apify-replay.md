# Progress Report - Session 041 Budget-Safe Apify Replay

Date: 2026-08-14
Session: 041-budget-safe-apify-replay
Agent: Codex

## Objective

Protect limited Apify trial usage with one-run batching, local dataset replay, default-off live guards, and focused fixes for the latest real candidate noise without another live crawl.

## Changes Made

- Kept one HTTP actor call per Discovery Crawl and extracted/tested the batched input containing all seed keywords.
- Cached successful raw actor dataset items and non-secret run metadata under ignored `data/cache/apify-last-run.json`.
- Added `replay_last_apify_crawl`, which reads cache, applies current entity-first filtering, saves included posts, and returns normal diagnostics without calling Apify.
- Added cache age messaging; stale caches remain replayable.
- Added `APIFY_LIVE_CRAWL_ENABLED=false`, `APIFY_MAX_LIVE_RUNS_PER_SESSION=1`, and `APIFY_CACHE_TTL_HOURS=24` defaults while retaining the 30-900 second timeout bound.
- Added replay controls and usage notes to the desktop UI. Apify single-seed testing is clearly labeled as a credit-consuming debug action.
- Rejected role/platform/generic candidates `Agent Engineer`, `AI Agent Engineer`, `Copilots`, and `YouTube`.
- Added domain evidence policy: launch/identity language and AI product context are both required for a domain candidate such as `folk.com`.

## Replay Validation

- Used five local cached fixtures representing `Claude Code`, `folk.com`, Agent Engineer, Copilots, and YouTube.
- Replay fetched 5, included/saved 2, and filtered 3 without network access.
- `Claude Code` remained a known alias.
- `folk.com` entered Candidate Review as one pending `unknown_candidate` due to explicit launch evidence.
- Agent Engineer, Copilots, and YouTube did not enter Candidate Review.
- Pending `folk.com` did not enter weekly metrics. After explicit approval in the test, it did enter metrics.

## Validation

- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check`: passed with existing warnings only.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.
- `cargo test validates_raw_post_insert_after_schema_init -- --test-threads=1`: passed.
- Entity detector suite: 24 passed.
- Apify connector suite: 6 passed.
- Cost classifier suite: 8 passed.
- Weekly targeted tests: 2 passed.
- Candidate-targeted tests: 10 passed.
- Reset-targeted test: passed.
- `git diff --check`: passed.
- Security scan found no real secret values; matches were literal command names in historical reports only.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low

## Risks / Notes

- Cache replay requires a cache created by a successful live run; none is committed.
- The live-run limit is process-local and resets after app restart.
- Domain evidence is intentionally conservative and may miss a newly launched tool described without launch/identity wording.
- No live Apify request was made in this session.

## Next Recommended Task

Review the replay UI with a real local cache if available, then decide whether the five local commits are ready to push. Keep live crawling disabled unless another paid run is explicitly necessary.
