# Dashboard UI Builder

## Purpose
Guide agents when building Svelte/TypeScript dashboard UI for the local-first desktop app.

## When to Use
Use this skill when adding or changing frontend screens, dashboard controls, report views, ingestion status, settings, or export UI.

## Inputs
- Current task objective.
- Existing files under `src/`.
- Backend command/service contracts if relevant.
- Current `AGENTS.md` rules.
- Any user-provided UX requirements.

## Rules
- Read `AGENTS.md` before making changes.
- Keep UI changes focused and consistent with the existing Svelte app.
- Do not implement backend behavior from the UI layer.
- Do not expose tokens, API keys, or secrets in visible UI or frontend bundles.
- Prefer simple TypeScript types and local components before adding UI libraries.
- Keep local-first desktop workflows obvious and reversible.
- Validate with `npm run build` after frontend changes.
- Update progress report, token usage log, and handoff note if work remains unfinished.

## Output Requirement
- Summarize UI files changed.
- Describe visible behavior before and after.
- State build/test results.
- Note any backend contract assumptions.

## Risk Note
Dashboard UI can imply working ingestion or reports before the backend exists. Keep placeholder states honest and avoid wiring fake production behavior.

## Completion Checklist
- [ ] `AGENTS.md` was followed.
- [ ] UI scope matches the requested task.
- [ ] No secret is exposed in frontend code.
- [ ] `npm run build` was run or a reason was documented.
- [ ] Progress report and token usage log were updated.
- [ ] Handoff note was updated if pending work remains.
