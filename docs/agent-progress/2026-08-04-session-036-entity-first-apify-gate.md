# Progress Report - Session 036 Entity-First Apify Gate

Date: 2026-08-04
Session: 036-entity-first-apify-gate
Agent: Codex

## Objective

Realign Apify discovery with the product's named entity radar intent: keep a post only when a concrete AI agent, skill, named MCP server, framework, or tool can be extracted.

## Changes Made

- Replaced the Apify AI/developer-context inclusion rule with an entity-first gate.
- Reused the authoritative alias configuration for known entity detection.
- Kept standalone `MCP` as context only for the Apify gate.
- Tightened unknown candidates to 1-3 word product-like names near agent/tool context.
- Added explicit support for strict candidates such as `Graphify`, `Headroom`, and CamelCase product names.
- Added entity-gate diagnostics, reason counts, included samples with detected entities, and filtered samples with reasons.
- Restricted weekly metrics to approved `known_alias` and `reviewed_candidate` mentions and excluded generic concepts including standalone `MCP`.
- Updated desktop UI and README wording to describe named entity discovery rather than generic AI Agent discussion collection.

## Validation

- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check`: passed with existing placeholder/dead-code warnings only.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.
- `cargo test validates_raw_post_insert_after_schema_init -- --test-threads=1`: passed.
- `cargo test requires_named_entities_for_apify_discovery -- --test-threads=1`: passed.
- `cargo test entity_gate_rejects_generic_concepts_and_accepts_named_entities -- --test-threads=1`: passed.
- `cargo test accepts_strict_brand_like_unknown_candidates -- --test-threads=1`: passed.
- `cargo test excludes_generic_concepts_and_threadbait_fragments_from_candidates -- --test-threads=1`: passed.
- `cargo test validates_weekly_metrics_group_canonical_entities_and_exclude_generic_mcp -- --test-threads=1`: passed.
- `git diff --check`: passed after code and documentation updates.
- Security grep found only historical documentation references to scan patterns; no token or secret values were found.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low

## Risks / Notes

- Candidate extraction is intentionally conservative. A niche single-word product without a brand-like shape may require an explicit alias or allowlist entry.
- `Graphify` passes by product-name affix; `Headroom` is explicitly allowlisted because it is also a common English word.
- Apify remains an experimental fallback and requires legal/compliance review before production use.

## Next Recommended Task

Run a real Apify crawl after clearing local demo data and compare entity-gate included/filtered totals, pending candidate quality, and weekly metrics noise against the previous live baseline.
