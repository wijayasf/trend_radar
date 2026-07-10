# Agent Progress Reporter

## Purpose
Guide agents when recording session progress, token usage, validation, risks, and handoff notes for AI Agent Trend Radar.

## When to Use
Use this skill at the end of every work session, after validation, or whenever a task is paused, blocked, or handed to another agent such as Claude.

## Inputs
- Current task objective.
- Files created or changed.
- Validation/test results.
- Known risks and pending tasks.
- Token usage data if available.
- Current `AGENTS.md` rules.

## Rules
- Read `AGENTS.md` before updating reports.
- Create a new progress report in `docs/agent-progress/` for each session.
- Update `docs/agent-progress/token-usage-log.md` every session.
- Use `Unknown` or `Estimated` when token start/used/remaining are unavailable.
- Update `docs/agent-handoff/latest-handoff.md` when work remains pending or context should transfer.
- Keep handoff notes concrete enough for Codex or Claude to continue.
- Do not include secrets, tokens, local database contents, or private user data.

## Output Requirement
- New or updated progress report with objective, changes, validation, token usage, risks, and next task.
- Updated token usage log row.
- Updated latest handoff note when applicable.
- Final response summary including files changed, validation, risks, next step, and token usage.

## Risk Note
Incomplete progress notes make future sessions repeat work or miss security constraints. Overly vague token reporting should be labeled as low accuracy.

## Completion Checklist
- [ ] `AGENTS.md` was followed.
- [ ] Progress report was created or updated.
- [ ] Token usage log was updated.
- [ ] Latest handoff was updated when needed.
- [ ] Validation results and risks are recorded.
- [ ] Final summary matches the actual changes.
