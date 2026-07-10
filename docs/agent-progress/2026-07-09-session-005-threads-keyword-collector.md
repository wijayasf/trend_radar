# Progress Report

Date: 2026-07-09
Session: 005-threads-keyword-collector
Agent: Codex

## Objective

Create a Threads API client MVP that searches public posts by keyword and stores raw results in DuckDB, without implementing sentiment, entity/region detection, dashboard analytics, or full pagination.

## Changes Made

- Added direct Rust dependencies for the Threads client:
  - `reqwest` with `blocking`, `json`, and `rustls-tls`.
  - `serde` with `derive`.
  - `serde_json`.
- Created `src-tauri/src/services/threads_client.rs`.
- Created `src-tauri/src/commands/threads.rs`.
- Created minimal Threads models in `src-tauri/src/models/threads.rs`.
- Registered Tauri command `collect_threads_by_keyword(keyword: String)`.
- Added keyword collector UI with input, collect button, status, fetched count, and saved count.
- Added `media_type` to `threads_posts_raw` schema and docs.
- Added simple schema compatibility line for existing local DBs: `ALTER TABLE threads_posts_raw ADD COLUMN IF NOT EXISTS media_type TEXT`.
- Added raw post insert logic in `duckdb_service`.
- Kept author/user data out of MVP storage; collector does not request author id or username.
- Sanitized request failure messaging so `reqwest` errors cannot accidentally return a URL containing `access_token`.
- Added clear TODO for pagination after endpoint contract and storage behavior are confirmed.

## Validation

- Attempted to read official Meta Threads API docs for Keyword Search at `https://developers.facebook.com/docs/threads/keyword-search/`, but the docs page returned HTTP 429 from the browsing tool. Implementation uses a defensively isolated `graph.threads.net` keyword search client and records this verification gap.
- `cargo add reqwest@0.12.28 --features blocking,json,rustls-tls --no-default-features` completed.
- `cargo add serde@1.0.228 --features derive` completed.
- `cargo add serde_json@1.0.150` completed.
- `npm run build` passed.
- `cargo fmt` was run.
- Final `cargo fmt --check` passed.
- Final `cargo check` passed with expected placeholder dead-code warnings.
- Security grep found no real hardcoded token/API key values. It found only env var names, safe docs guidance, and token-related keyword config text.
- Security review found and fixed a potential token leak path in raw request error formatting.
- `.env` was not created.
- `find . -name '*.duckdb' -o -name '*.duckdb.*'` found no local DuckDB database files created during validation.
- No real Threads API call was made because no token was configured or used.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable in this session
- Accuracy: Low

## Risks / Notes

- Official Threads Keyword Search docs could not be read due HTTP 429, so endpoint parameter details should be re-verified before using a real token.
- The MVP assumes `https://graph.threads.net/v1.0/keyword_search` with `q`, `fields`, and `access_token` query parameters.
- Full pagination is not implemented yet.
- Rate limiting is handled defensively through HTTP 429 and API error messages, but should be validated against real API responses.
- Raw JSON is stored locally; author id and username are intentionally not requested or stored in this MVP.
- Request transport failures return a generic message to avoid exposing `access_token` query strings.
- `cargo check` still reports expected dead-code warnings for placeholder models/services.
- Disk space remains a concern because DuckDB bundled builds are large.

## Next Recommended Task

Verify the Threads Keyword Search endpoint against official docs or a safe real-token dry run, then add pagination and a small manual collector test flow using non-sensitive keywords such as `Claude Code`, `Cursor AI`, and `AI Agent`.
