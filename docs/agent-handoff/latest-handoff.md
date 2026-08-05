# Latest Handoff

Date: 2026-08-05
Session: 039-non-tool-noise-cleanup
Agent: Codex

## Current State

The named-entity pipeline now blocks the remaining live-crawl platform and recruitment noise. Weekly dashboard/export queries show only the latest week, and free usage credits are recognized as a positive cost signal.

## Key Changes

- Explicitly rejected `GenAI`, social platforms, appointment setter/closer roles, and related phrases as unknown candidates.
- Added Apify filter reason `recruitment_or_job_post`; recruitment posts still pass when a concrete named tool such as `Claude Code` is present.
- Made known aliases authoritative within a post while retaining strong product-shaped candidates such as `Graphify` and `Headroom`.
- Limited ranking and export loaders to the maximum `week_start`, preventing historical `Claude Code` rows from appearing as duplicates.
- Classified free usage/account credit phrases as `cost_positive`.
- Made Apify included/filtered diagnostics wrap long metadata and snippets cleanly.

## Validation Snapshot

- Frontend production build, Rust format/check, full sample flow, and raw insert regression passed.
- All 22 entity detector tests, 3 Apify tests, 8 cost tests, and 2 weekly tests passed.
- Existing Rust placeholder/dead-code warnings remain unchanged.
- Security scan found only literal pattern names in historical progress notes, with no secret values; ignored runtime files remain untracked and `src-tauri/data` is empty.

## Pending

- Run a fresh live Apify crawl to confirm the four observed candidates no longer appear in Candidate Review.
- Push the four local quality commits only when explicitly requested.

## Risk Note

The new filters are deterministic and covered by real-snippet tests, but the latest live Apify dataset has not been fetched again after this patch. Recruitment posts that mention a concrete named tool are intentionally retained for product-signal review.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
