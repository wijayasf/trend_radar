# DuckDB Analytics Designer

## Purpose
Guide agents when designing local DuckDB storage, analytics schemas, migrations, and query boundaries for AI Agent Trend Radar.

## When to Use
Use this skill when defining tables, local analytics flow, report source data, DuckDB access patterns, or storage documentation.

## Inputs
- Current task objective.
- Existing config files under `config/`.
- Planned report requirements.
- Expected local database path from `.env.example`.
- Current `AGENTS.md` rules.

## Rules
- Read `AGENTS.md` before making changes.
- Start with schema documentation before adding DuckDB code or crates.
- Keep schema changes focused on current report needs.
- Do not store secrets, raw access tokens, or unnecessary personal data.
- Prefer append-friendly event/history tables for trend analysis.
- Separate raw Threads records, normalized entities, scoring artifacts, and generated report metadata.
- Do not create a local database file unless the task explicitly asks for runtime validation.
- Update progress report, token usage log, and handoff note if work remains unfinished.

## Output Requirement
- Provide the proposed schema or storage boundary.
- List assumptions about retention, deduplication, and local-first behavior.
- Identify validation performed, such as doc review or SQL syntax check if applicable.
- Name the next implementation step.

## Risk Note
Poor schema boundaries can make trend scoring, deduplication, and report reproducibility difficult. Avoid prematurely optimizing before ingestion and reporting requirements are clearer.

## Completion Checklist
- [ ] `AGENTS.md` was followed.
- [ ] No database file or secret was committed.
- [ ] Schema design supports local-first storage.
- [ ] Raw, normalized, scoring, and reporting concerns are separated.
- [ ] Progress report and token usage log were updated.
- [ ] Handoff note was updated if pending work remains.
