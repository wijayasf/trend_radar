# Progress Report - Session 040 Live Apify Release Gate

Date: 2026-08-14
Session: 040-live-apify-release-gate
Agent: Codex

## Objective

Run one final fresh live Apify crawl and push the four local quality commits only if Candidate Review, latest-week metrics, cost classification, build validation, and security checks are clean.

## Execution

- `npx tauri dev` compiled and launched the backend process successfully.
- macOS reported zero windows for the native process, so UI click automation could not proceed.
- Ran the same Apify connector and downstream services against a fresh isolated DuckDB in `/tmp`, using the UI default eight seeds and max posts `10`.
- Removed the temporary ignored live-test harness after collecting the safe metrics and snippets.

## Fresh Live Crawl

- Fetched: 38.
- Entity-gate included: 5.
- Entity-gate filtered: 33.
- Saved unique posts: 5.
- Duplicates: 0.
- Filtered reasons: no named entity 23, recruitment/job 2, generic MCP 2, generic AI Agent 6, threadbait 0, ambiguous 0, empty 0.

Included entities were `Agent Engineer`, `folk.com`, `Copilots`, `Claude Code`, and `YouTube`. The first, third, and fifth are clear non-tool noise. `folk.com` is a concrete domain but is borderline and needs explicit product-review policy.

## Pipeline Metrics

- Raw posts: 5.
- Mentions found/saved: 6/6.
- Pending candidates: 4.
- Approved decisions: 0.
- Ignored decisions: 0.
- Region: Indonesia 0, Global 5, Unknown 0.
- Cost: positive 0, mixed 1, negative/boros 0, not mentioned 5.
- Weekly rows: 1; Top Indonesia 0, Top Global 1, Top Unknown 0.
- Latest-week ranking contained one `Claude Code` row with one mention and no pending candidates.

## Quality Decision

Do not push. Candidate Review is not clean even though weekly ranking is correctly protected from pending candidates and duplicate canonical rows.

## Validation

- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo check`: passed with existing warnings only.
- `cargo test validates_sample_full_mvp_flow -- --test-threads=1`: passed.
- `cargo test validates_raw_post_insert_after_schema_init -- --test-threads=1`: passed.
- Entity detector suite: 22 passed.
- Apify connector suite: 3 passed.
- Cost classifier suite: 8 passed.
- Weekly targeted tests: 2 passed.
- Candidate-targeted tests: 9 passed.
- Reset-targeted test: passed.
- `git diff --check`: passed before session documentation updates.
- Security scan found no real secret values.

## Risks / Notes

- The live crawl is time-dependent and may return different posts on the next run.
- A broad domain rejection would remove `folk.com` but could also hide legitimate emerging tools. A stronger candidate-evidence rule is safer than denying all domains.
- The `$100 in free usage credits` fixture remains covered by a passing unit test but was not present in this live dataset.

## Next Recommended Task

Add focused tests and filters for role-title candidates, generic plural concepts, and known platforms. Define a minimal domain-evidence policy for borderline candidates, then repeat this release gate.
