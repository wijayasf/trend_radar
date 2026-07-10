# Report Generator

## Purpose
Guide agents when designing or implementing weekly report generation and export for AI Agent Trend Radar.

## When to Use
Use this skill when creating report schemas, ranking outputs, Markdown/HTML/CSV exports, weekly report summaries, or report storage under `docs/reports/`.

## Inputs
- Current task objective.
- Report requirements for Top 20 Indonesia, Top 20 Global, sentiment, cost signals, and emerging tools.
- Planned DuckDB schema or query outputs.
- Current `AGENTS.md` rules.

## Rules
- Read `AGENTS.md` before making changes.
- Keep report output reproducible from local stored data.
- Do not include secrets or raw tokens in reports.
- Make report timestamps, data windows, and scoring assumptions explicit.
- Keep export format simple until real user needs require more.
- Do not invent analytics results without source data.
- Treat exported reports as user data.
- Update progress report, token usage log, and handoff note if work remains unfinished.

## Output Requirement
- State report format, location, and data window.
- List sections produced or designed.
- Document scoring/query assumptions.
- Note validation performed and missing data dependencies.

## Risk Note
Reports can look authoritative even when source data is incomplete. Always surface data window, source coverage, and scoring assumptions.

## Completion Checklist
- [ ] `AGENTS.md` was followed.
- [ ] Report does not expose secrets.
- [ ] Output is tied to local data or clearly marked as template/design.
- [ ] Data window and assumptions are documented.
- [ ] Progress report and token usage log were updated.
- [ ] Handoff note was updated if pending work remains.
