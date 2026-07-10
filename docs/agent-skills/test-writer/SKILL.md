# Test Writer

## Purpose
Guide agents when adding focused tests or validation checks for AI Agent Trend Radar.

## When to Use
Use this skill when adding tests for Rust services/models, frontend behavior, config parsing, schema logic, classifiers, scoring, or report generation.

## Inputs
- Current task objective.
- Changed files or behavior under test.
- Existing test setup, if any.
- Current `AGENTS.md` rules.
- Known risks from progress/handoff notes.

## Rules
- Read `AGENTS.md` before making changes.
- Match test scope to risk and blast radius.
- Prefer deterministic local tests.
- Do not call real Threads API or remote LLMs in automated tests.
- Do not require real secrets, `.env`, or local user databases.
- Avoid adding test frameworks until needed by the current stack and task.
- Run the narrowest relevant validation command first, then broader checks when warranted.
- Update progress report, token usage log, and handoff note if work remains unfinished.

## Output Requirement
- List tests added or validation checks performed.
- Explain what behavior is covered.
- State commands run and results.
- Document any test gaps that remain.

## Risk Note
Tests that depend on live APIs, local secrets, or generated user data will be flaky and unsafe. Keep tests isolated and reproducible.

## Completion Checklist
- [ ] `AGENTS.md` was followed.
- [ ] Tests are scoped to the changed behavior.
- [ ] No live API, secret, or local database dependency was introduced.
- [ ] Relevant validation command passed or failure is documented.
- [ ] Progress report and token usage log were updated.
- [ ] Handoff note was updated if pending work remains.
