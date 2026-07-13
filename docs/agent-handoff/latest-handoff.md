# Latest Handoff

Date: 2026-07-13
Session: 025-real-discovery-smoke
Agent: Codex

## Current State

Real Threads discovery smoke test has been run successfully at the backend/service layer.

The app can:

1. Run real keyword search.
2. Receive ID-only keyword search results.
3. Fetch post detail for the ID.
4. Store raw post detail with `text_missing = false`.
5. Run entity detection safely even when no configured alias is found.

## Real Smoke Result

- `seeds_processed`: `21`
- `fetched_total`: `1`
- `id_only_results_count`: `1`
- `detail_fetched_total`: `1`
- `detail_failed_total`: `0`
- `text_missing_total`: `0`
- `saved_total`: `1`
- `failed_seeds`: `0`
- `mode`: `real_threads`

Entity detection result:

- `analyzed_posts`: `1`
- `mentions_found`: `0`
- `saved_count`: `0`

Safe diagnostic:

- `raw_post_count`: `1`
- `text_missing_count`: `0`
- raw JSON top-level keys: `id`, `media_type`, `owner`, `permalink`, `text`, `timestamp`, `username`

## Important Finding

The real detail endpoint rejected the `caption` field with `code=100`. The request field list was adjusted to:

```text
id,text,media_type,permalink,timestamp,username,owner
```

The parser still tolerates `caption` if a payload contains it, but the real request no longer asks for it.

## Validation Commands

- `npx tauri dev`: launched successfully.
- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check`: passed.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.
- `cargo test validates_real_threads_discovery_smoke -- --ignored --nocapture --test-threads=1`: passed.

## Known Gaps

- Native UI click flow was not automated because macOS Assistive Access is required.
- Real weekly metrics were not produced because the real fetched post did not contain a configured detectable entity.
- Public search scope may still be limited by Threads app approval.

## Suggested Next Step

Create or verify tester-owned Threads posts containing `Ponytail`, `Cavemen`, `Astryx`, `Claude Code`, and `Cline`, then rerun real discovery smoke and manual UI validation.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
