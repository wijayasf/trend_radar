# Latest Handoff

Date: 2026-07-13
Session: 029-duckdb-schema-compat-fix
Agent: Codex

## Current State

The DuckDB runtime error involving `agent_mentions_compatible` has been fixed.

## Root Cause

`agent_mentions_compatible` was a stale temporary compatibility migration table used to recreate `agent_mentions`.

The migration was still executed on every schema initialization. Raw post save calls schema initialization first, so the compatibility migration failure appeared as:

```text
DuckDB raw post insert failed: Catalog Error: Table with name agent_mentions_compatible does not exist!
```

## Fix

- Removed the `agent_mentions_compatible` migration.
- Removed all source/runtime references to `agent_mentions_compatible`; documentation keeps historical notes for this fix.
- Schema initialization now relies on additive `CREATE TABLE IF NOT EXISTS` and `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` statements.
- No local DuckDB deletion is required.

## Validation

- `grep -R "agent_mentions_compatible" -n src-tauri/src docs README.md || true`: source/runtime references removed; remaining matches are documentation notes for this fix.
- `cargo test validates_raw_post_insert_after_schema_init -- --test-threads=1`: passed.
- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check`: passed.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.

## Pending

- Manual UI confirmation:
  1. `npx tauri dev`
  2. Run Discovery Crawl or Collect raw posts
  3. Confirm no missing compatibility table error

## Risk Note

- The MVP schema path avoids destructive table recreation now.
- If a much older local DB has stale CHECK constraints, a targeted non-destructive migration may still be needed later.
- Existing Rust placeholder dead-code warnings remain.
- Token and `.env` contents were not read or printed.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
