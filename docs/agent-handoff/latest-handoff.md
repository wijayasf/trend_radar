# Latest Handoff

Date: 2026-08-05
Session: 038-candidate-fragment-suppression
Agent: Codex

## Current State

Known-alias fragments and generic Large Action Model terms no longer enter Candidate Review. The 42-post real-data replay now produces zero pending candidates while retaining the same nine canonical weekly entities.

## Key Changes

- `Claude Code`, `Codex CLI`, and `GitHub Copilot` are detected only as full known entities; `Code`, `CLI`, `GitHub`, `Copilot`, `Claude`, and `Codex` are suppressed as unknown fragments.
- Generic concepts including `LAMs`, `LLMs`, Large Action Models, standalone `MCP`, `API`, `SDK`, and `HTML` cannot become unknown candidates.
- `Graphify` and `Headroom` remain valid pending candidates.
- Apify enforces at least 10 max posts and uses a configurable 300-second default timeout with a friendly timeout message.
- Schema initialization removes a real legacy compatibility table/view. Phantom DuckDB metadata returns a safe manual local-reset message; the database file is never deleted automatically.

## Validation Snapshot

- Baseline: 42 raw / 103 mentions / 70 pending.
- Previous entity gate: 16 included / 22 mentions / 4 pending / 9 weekly rows.
- Current replay: 15 included / 17 mentions / 0 pending / 9 weekly rows.
- Entity, Apify, reset, compatibility cleanup, full-flow, raw insert, weekly grouping, frontend build, Rust format/check, and diff validations passed.
- Existing Rust dead-code warnings remain unchanged.
- Security scan found no token or secret values.
- `src-tauri/data` remains empty.

## Pending

- Run one fresh live Apify crawl with the new 300-second timeout.
- Review the live included/filtered samples and confirm actor latency is acceptable.
- Push only when explicitly requested.

## Risk Note

Live Apify completion under the new timeout has not yet been revalidated. The root local DuckDB may still need manual removal for a clean demo if its old phantom metadata prevents reset.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
