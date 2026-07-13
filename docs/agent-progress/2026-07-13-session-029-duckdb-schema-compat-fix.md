# Progress Report: Session 029 - DuckDB Schema Compatibility Fix

Date: 2026-07-13
Agent: Codex

## Objective

Fix UI/runtime failure during discovery crawl or raw post collection:

```text
DuckDB raw post insert failed: Catalog Error: Table with name agent_mentions_compatible does not exist!
```

## Root Cause

`agent_mentions_compatible` was a stale temporary migration table used by an older compatibility migration that recreated `agent_mentions`.

That migration was still executed during every database initialization. Because raw post save calls `initialize_database()` first, a schema migration failure could surface during raw post insert even though raw post insert itself does not need `agent_mentions_compatible`.

## Completed

- Removed all source references to `agent_mentions_compatible`.
- Removed the destructive compatibility migration that created, copied, dropped, and renamed mention tables.
- Kept schema initialization idempotent through:
  - `CREATE TABLE IF NOT EXISTS`
  - `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`
- Added regression test:
  - initialize temp DuckDB
  - save raw post
  - count raw posts
  - run entity detection
  - verify no dependency on the removed compatibility migration
- Updated schema docs and handoff notes.

## Validation

- `grep -R "agent_mentions_compatible" -n src-tauri/src docs README.md || true`: source/runtime references removed; remaining matches are documentation notes for this fix.
- `cargo test validates_raw_post_insert_after_schema_init -- --test-threads=1`: passed.
- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check`: passed.
  - Existing placeholder dead-code warnings remain.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.

## Local DB Migration Note

No local DB deletion is required. On next app start, schema initialization will run additive migrations against `agent_mentions` directly and will no longer reference `agent_mentions_compatible`.

## Risk Note

- This avoids destructive table recreation for MVP safety.
- If an old database has outdated CHECK constraints, a future explicit migration may still be needed. Current app/test flows pass with additive schema initialization.
- Token and `.env` contents were not read or printed.

## Next Recommended Task

Run `npx tauri dev`, then try `Run Discovery Crawl` or `Collect raw posts` once to confirm the UI no longer shows the missing compatibility table error.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
