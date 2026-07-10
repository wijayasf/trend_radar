# Security Reviewer

## Purpose
Guide agents when reviewing AI Agent Trend Radar for secret handling, local data safety, API boundaries, and desktop app security risks.

## When to Use
Use this skill before adding network calls, credential loading, storage access, report export, or release packaging; also use it when asked to review security.

## Inputs
- Current task objective or diff scope.
- `AGENTS.md`.
- `.env.example`.
- Relevant code/config/docs changed in the session.
- Validation results from the implementing agent.

## Rules
- Read `AGENTS.md` before reviewing.
- Lead with concrete findings and file references.
- Check for hardcoded tokens, API keys, user IDs, credentials, local database leaks, and report data exposure.
- Check `.gitignore` coverage for `.env`, local databases, build artifacts, and generated sensitive files.
- Do not broaden the review beyond the requested scope unless a severe issue is visible.
- Do not add security dependencies unless the task explicitly requires implementation.
- Update progress report, token usage log, and handoff note if work remains unfinished.

## Output Requirement
- Findings first, ordered by severity.
- Include file paths and line references when possible.
- State if no issues were found.
- List residual risks and recommended next checks.

## Risk Note
False confidence is dangerous in security review. Distinguish verified findings from assumptions, and call out areas not inspected.

## Completion Checklist
- [ ] `AGENTS.md` was followed.
- [ ] Secrets and local data paths were checked.
- [ ] Findings include actionable references or state no issues found.
- [ ] Residual risks are documented.
- [ ] Progress report and token usage log were updated.
- [ ] Handoff note was updated if pending work remains.
