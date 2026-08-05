# Progress Report - Session 039 Non-Tool Noise Cleanup

Date: 2026-08-05
Session: 039-non-tool-noise-cleanup
Agent: Codex

## Objective

Remove the remaining platform/recruitment false positives from Apify discovery, prevent historical weekly rows from looking like duplicates, recognize free usage credits, and improve diagnostic-card readability.

## Changes Made

- Added explicit unknown-candidate exclusions for `GenAI`, `TikTok`, `Threads`, `Instagram`, setter/closer roles, and combined variants.
- Suppressed weak unknown candidates whenever a known alias is already present, while preserving strict names such as `Graphify` and `Headroom`.
- Added an Apify recruitment/job filter with the diagnostic reason `recruitment_or_job_post`. A concrete detected named entity remains an intentional exception.
- Updated dashboard and export metric loaders to select the maximum available `week_start`; historical rows stay stored.
- Added positive cost indicators for free usage, usage credits, free/account credits, and `$100 in free` wording.
- Made Apify included/filtered preview cards use a single-column layout with wrapped metadata.

## Before / After Expectation

- Before: live Candidate Review included `GenAI`, `Setter/Closer`, `Threads TikTok Instagram`, and `TikTok`.
- After: those strings are rejected; named entities such as `Graphify`, `Headroom`, `Claude Code`, and `Ponytail` remain eligible.
- Before: Top Global could show multiple `Claude Code` rows from different weeks.
- After: latest-week loaders return one aggregated canonical row; the regression fixture produces one `Claude Code` row with four mentions.
- Before: `$100 in free usage credits` was `not_mentioned`.
- After: it is `cost_positive`.

## Validation

- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check`: passed with existing placeholder/dead-code warnings only.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.
- `cargo test validates_raw_post_insert_after_schema_init -- --test-threads=1`: passed.
- Entity detector suite: 22 passed.
- Apify connector suite: 3 passed.
- Cost classifier suite: 8 passed.
- Weekly targeted tests: 2 passed.
- `git diff --check`: passed.
- Security scan found no token or secret values; ignored runtime files remain untracked and `src-tauri/data` is empty.

## Token Usage

- Start: Unknown
- Used: Estimated
- Remaining: Unknown
- Source: Codex goal metadata unavailable
- Accuracy: Low

## Risks / Notes

- No fresh paid/live Apify actor run was started after the patch; exact post counts therefore remain dependent on the next live crawl.
- Recruitment posts with a known concrete entity are retained by design.
- Historical weekly rows remain in DuckDB and can support a later explicit week selector.

## Next Recommended Task

Run one fresh Apify UI crawl, confirm the observed four false positives stay absent, then push the four local quality commits if the samples remain clean.
