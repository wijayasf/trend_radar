# Latest Handoff

Date: 2026-08-04
Session: 036-entity-first-apify-gate
Agent: Codex

## Current State

Apify discovery now follows an entity-first rule. Generic AI Agent/MCP discussion is filtered before raw storage unless the shared entity detector finds a known concrete alias or a strict product-like unknown candidate.

## Key Changes

- `NamedEntityGateDetector` reuses `config/aliases.yml` and strict unknown-candidate extraction.
- Standalone `MCP` is context only for the Apify gate and weekly ranking.
- Unknown names require a 1-3 word product shape and nearby AI/tool context; accepted unknown names remain pending until reviewed.
- Apify diagnostics expose gate included/filtered totals, reason counts, included samples with entity names, and filtered samples with reasons.
- Weekly metrics include only approved `known_alias` and `reviewed_candidate` mentions.
- UI and README now describe the product as a named entity radar.

## Validation

- Frontend build, Rust formatting/check, full-flow test, raw insert regression, new entity-gate tests, strict-candidate test, weekly generic-MCP exclusion test, and diff validation passed.
- Existing Rust placeholder/dead-code warnings remain unchanged.
- Secret scans found only historical documentation references to scan patterns; no token or secret values were found.

## Pending

- Run a real Apify crawl after `Clear Local Demo Data` to measure live precision.
- Do not push unless explicitly requested.

## Risk Note

- Conservative extraction may miss niche plain single-word product names; add explicit aliases or narrowly reviewed allowlist entries when evidence supports them.
- Apify remains experimental and needs compliance review before production use.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low
