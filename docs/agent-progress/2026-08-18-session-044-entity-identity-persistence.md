# Session 044 - Entity Identity Persistence

Date: 2026-08-18
Agent: Codex

## Objective

Checkpoint the accepted IMP-01 multi-source foundation, then add persistent canonical aliases and append-only external identity review history without changing the existing detector, Candidate Review, collector, weekly metrics, or score formula.

## Checkpoint

- Revalidated IMP-01 with frontend build, Rust format/check, 64 passing tests, one ignored live-network test, diff validation, and secret review.
- Committed the nine IMP-01 files as `bc4d57c feat: add multi-source identity foundation`.
- Created branch `feature/entity-identity-persistence` from that checkpoint.

## Changes

- Added additive `entity_aliases` storage with scoped uniqueness, provenance, ambiguity context, archive status, and source-aware multi-candidate lookup.
- Added explicit, idempotent bootstrap from the real `config/aliases.yml`; detector behavior still reads YAML directly.
- Added append-only `external_identity_reviews` repository history with approved/rejected/ambiguous decisions.
- Added transactional review behavior that appends audit history, updates effective link state, and reconciles source-record resolution atomically.
- Added collision, source-scope, ambiguity, review reversal, rollback, same-name rejection, child-resource, multi-entity editorial, and IMP-01 upgrade tests.
- Documented the DuckDB parent-update foreign-key limitation that requires audit referential integrity to be enforced inside the transaction rather than through audit-table foreign keys.

## Bootstrap Result

- Curated entities: 26.
- Canonical entities created on first run: 26.
- Aliases persisted on first run: 63.
- Ambiguous aliases: 16.
- Type mappings: 11 agent tools, 8 framework SDKs, 3 app builders, 2 skills/modes, 1 protocol, and 1 registry/discovery entity.
- Skipped or unsupported: 0.
- Second run: 26 entities reused, 63 aliases found existing, 0 entities or aliases created.

## Compatibility

- Legacy MVP data survives additive initialization.
- Existing IMP-01 entities and source records survive IMP-02 initialization.
- No `agent_mentions.entity_id` column was added.
- No ExplainX ingestion, collector, UI, scoring, or weekly aggregation changes were made.
- Candidate Review semantics remain unchanged.

## Validation

- Focused repository tests: 18 passed, 0 failed.
- Real YAML bootstrap test: passed.
- Default-parallel Rust suite: passed twice with 72 passed, 0 failed, and 1 live-network test ignored.
- Serial Rust suite: 72 passed, 0 failed, and 1 live-network test ignored.
- Frontend production build, Rust format/check, diff check, and secret scan passed.

## Parallel Test Isolation

- Root cause: eight DB-backed tests mutated process-global `DATABASE_PATH`, so parallel tests could open or clean another test's DuckDB/WAL files.
- Added a test-only thread-local database path override in the config resolver.
- Updated the main integration-style tests and Apify cache replay test to use scoped path guards instead of environment mutation.
- Production database-path behavior is unchanged; every affected test continues to use its own named temporary database.
- Threads mock environment mutation remains limited to the full-flow test and does not control database selection.

## Risk Note

- External review audit rows intentionally lack DuckDB foreign keys because inserting the audit row before updating its referenced effective link triggers DuckDB's parent-update limitation. The repository validates and copies all referenced IDs inside one transaction.
- The bootstrap is deliberately service-only and explicit; no command or startup hook invokes it yet.
- Test database isolation is thread-local under `cfg(test)`; future DB-backed tests must use the scoped helper rather than mutating `DATABASE_PATH`.

## Next Recommended Task

Review and approve IMP-02 before designing the IMP-03 migration that associates detected mentions with canonical entity IDs.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
