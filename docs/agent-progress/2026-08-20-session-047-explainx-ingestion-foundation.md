# Session 047 - ExplainX Ingestion Foundation

Date: 2026-08-20
Agent: Codex

## Objective

Implement IMP-05 as an import-first ExplainX source foundation that stores structured local records and connects them conservatively to the existing multi-source identity model without adding scoring or live collection.

## Schema Changes

- Added additive `explainx_records` storage with UUID identity, unique source record key, normalized metadata, raw JSON, seen timestamps, ingestion batch UUID, and active/archive status.
- Added indexes for normalized-name lookup and source-record lookup.
- Kept referential validation to `source_records` at the service boundary to avoid DuckDB parent-update limitations.
- No existing table, row, classifier field, or metric is removed or rewritten.

## Import Behavior

- Added `import_explainx_records(file_path)` for absolute or project-root-relative local JSON files.
- Accepts a top-level array, validates each item as an object with a usable name, and supports common snake/camel-case identity and metadata keys.
- Retains each full object as `raw_json`, normalizes names, stores tags as JSON, and creates an ExplainX import collection run plus source observation.
- Uses a deterministic source key from explicit IDs/keys/slugs/paths/URLs, with a category/type/name fallback when the source omits an identifier.
- Re-import never duplicates source or ExplainX rows. Unchanged payloads count as skipped; changed payloads update the existing ExplainX row.
- Friendly errors cover missing files, invalid JSON, empty arrays, unsupported top-level shapes, and datasets with no valid named record.

## Source Identity Linkage

- Curated YAML aliases bootstrap idempotently before identity lookup.
- A single active, non-ambiguous exact alias creates a pending `same_entity` link; it is reported as an exact alias candidate but is not approved automatically.
- Child-resource signals such as MCP server, skill, plugin, or command create a pending `child_resource` link and remain review-needed.
- Multiple candidates or an alias marked ambiguous create no link and remain review-needed.
- Missing aliases remain unlinked.
- Existing explicit approved external reviews remain effective. Candidate Review stays independent.

## UI Changes

- Added a compact `ExplainX Import` panel with local file path input and loading state.
- Displays imported, inserted, updated, unchanged, invalid, exact-link, review-needed, and unlinked counts.
- Preview shows source key, name, category, tags, identity status, canonical candidate, and reason.

## Test Coverage

- Valid JSON persists normalized fields and raw JSON.
- A second identical import creates no duplicate rows; changed metadata updates one row.
- Invalid JSON and unsupported/empty shapes fail before persistence.
- Missing-name rows increment invalid while valid rows still import.
- Claude Code creates one safe pending exact-alias candidate.
- Codex remains ambiguous with no automatic canonical link.
- A Claude Code MCP-server record remains a pending child-resource review.
- A new unknown ExplainX tool remains unlinked.
- Candidate Review and canonical weekly metrics remain unchanged.
- Tests use explicit, isolated temp DuckDB paths and introduce no environment mutation or network call.

## Regression Result

- Targeted ExplainX tests: 4 passed, 0 failed.
- Frontend production build, Rust formatting, and `cargo check`: passed with seven unchanged dead-code warnings.
- Default-parallel Rust suite: 84 passed, 0 failed, 1 live-network test ignored.
- Serial Rust suite: 84 passed, 0 failed, 1 live-network test ignored.
- `git diff --check`: passed.
- No live ExplainX, Threads, or Apify request ran.

## Security Result

- Secret-pattern scans found no real Apify/Threads token or app secret; matches were limited to historical documentation naming the scan patterns.
- `.env`, local DuckDB files, caches, exports, `dist`, `node_modules`, and Rust target artifacts remain untracked.
- No runtime data or generated build output is included in IMP-05.

## Explicitly Not Implemented

- Live ExplainX scraping or API collection.
- Automatic approval or merge of external identities.
- External identity review UI/actions.
- ExplainX contribution to weekly metrics, cross-source scoring, WoW, velocity, momentum, or Programming Fit.
- LLM classification or fuzzy identity matching.

## Risks

- Source key fallback can collide when an upstream record lacks every stable identifier and repeats the same type/category/name; explicit ExplainX keys remain preferred.
- DuckDB cannot update a referenced `source_records` parent row reliably. Current mutable metadata is therefore authoritative in `explainx_records`, while source observations retain later seen history.
- Exact alias candidates are pending links, not reviewed identity decisions; a later external review workflow is still required.

## Recommended Next Step

Review and checkpoint IMP-05. The next scoped milestone should expose explicit external identity review for pending ExplainX links before any ExplainX-derived aggregation or scoring.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex runtime does not expose exact token accounting
- Accuracy: Low
