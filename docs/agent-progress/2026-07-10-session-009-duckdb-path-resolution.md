# Progress Report: Session 009 - DuckDB Path Resolution

Date: 2026-07-10
Agent: Codex

## Objective

Fix the root cause where `DATABASE_PATH=./data/app.duckdb` was resolved relative to `src-tauri`, causing DuckDB runtime files to be created under `src-tauri/data` and triggering Tauri dev watcher rebuild loops.

## Completed

- Reviewed database/config path handling in:
  - `src-tauri/src/utils/config.rs`
  - `src-tauri/src/services/duckdb_service.rs`
  - `.env.example`
  - `.gitignore`
- Added `resolved_database_path()` in `src-tauri/src/utils/config.rs`.
- Relative `DATABASE_PATH` values now resolve from the project root derived from `CARGO_MANIFEST_DIR/..`.
- `duckdb_service::configured_database_path()` now delegates to the shared config resolver.
- Added guardrail that rejects resolved paths inside `src-tauri/data` with:
  - `Invalid database path: runtime database must not be stored inside src-tauri`
- Updated `.env.example` with a note that relative database paths are resolved from the project root, not `src-tauri`.
- Removed wrong-location runtime files:
  - `src-tauri/data/app.duckdb`
  - `src-tauri/data/app.duckdb.wal`
  - `src-tauri/data/app.duckdb.tmp`

## Before Behavior

- `DATABASE_PATH=./data/app.duckdb` could be resolved relative to the Rust/Tauri working directory.
- Tauri dev created `src-tauri/data/app.duckdb`.
- DuckDB WAL updates under `src-tauri/data` triggered rebuild loops.

## After Behavior

- `DATABASE_PATH=./data/app.duckdb` resolves to root project `data/app.duckdb`.
- Any path under `src-tauri/data` is rejected before opening DuckDB.
- Wrong-location runtime files are absent after cleanup.

## Validation

- `cargo fmt` passed.
- `cargo fmt --check` passed.
- `npm run build` passed.
- Confirmed `src-tauri/data` has no runtime database files after cleanup.
- Confirmed `.gitignore` includes root `data/*.duckdb`, `data/*.duckdb.wal`, `data/*.duckdb.tmp`, and matching `src-tauri/data` patterns.
- `cargo check` was attempted after the previous session cleaned `src-tauri/target`, but failed while rebuilding `libduckdb-sys` because the system ran out of disk space:
  - `ranlib: can't write to output file (No space left on device)`
  - `ar: internal ranlib command failed`
- Cleaned the failed partial Rust build artifacts with `cargo clean`.
- Final disk check after cleanup: `10Gi` available.

## Not Completed

- `cargo check` could not complete because bundled DuckDB rebuild exceeded available disk space.
- `npm run tauri dev` was not run because backend validation did not complete and another Tauri rebuild would likely fill the disk again.
- Database health in the running Tauri app was not revalidated in this session.

## Files Changed

- `.env.example`
- `src-tauri/src/utils/config.rs`
- `src-tauri/src/services/duckdb_service.rs`
- `docs/agent-progress/2026-07-10-session-009-duckdb-path-resolution.md`
- `docs/agent-progress/token-usage-log.md`
- `docs/agent-handoff/latest-handoff.md`

## Risk Note

- The code path fix is in place, but runtime verification still needs a successful Rust rebuild.
- Bundled DuckDB is large; the machine needs more free disk space before `cargo check` or `npm run tauri:dev` can reliably complete after `cargo clean`.
- Next Rust validation will rebuild dependencies because partial artifacts were cleaned again.

## Next Recommended Task

Free additional disk space outside this project, then run `cargo check` and `npm run tauri:dev`. Verify the app creates `data/app.duckdb` in the project root and no `src-tauri/data/app.duckdb*` files appear.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
