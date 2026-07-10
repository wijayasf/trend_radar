# Threads API Integrator

## Purpose
Guide agents when planning or implementing Threads API integration behind a safe backend service boundary.

## When to Use
Use this skill when adding Threads API configuration, request clients, pagination, rate-limit handling, ingestion jobs, or API response models.

## Inputs
- Current task objective.
- Threads API documentation or endpoint requirements supplied for the task.
- `.env.example` placeholders.
- Existing `src-tauri/src/services/threads.rs`.
- Current `AGENTS.md` rules.

## Rules
- Read `AGENTS.md` before making changes.
- Never hardcode access tokens, API keys, app secrets, user IDs, or credentials.
- Load credentials only from local configuration or environment when implementation begins.
- Keep API logic behind a service interface.
- Add only minimal models needed for the current endpoint.
- Handle pagination, rate limits, and API errors explicitly when real requests are implemented.
- Do not call the real Threads API in tests unless the user explicitly requests it and provides a safe test setup.
- Update progress report, token usage log, and handoff note if work remains unfinished.

## Output Requirement
- Describe endpoint scope and service boundary.
- List configuration required from the user.
- State whether any real network call was made.
- Document validation and remaining API risks.

## Risk Note
Threads API integration can leak credentials or create brittle behavior if tokens, pagination, rate limits, and permission errors are not handled carefully.

## Completion Checklist
- [ ] `AGENTS.md` was followed.
- [ ] No credential or secret was added to source/docs/logs.
- [ ] API behavior is behind a service interface.
- [ ] Error and rate-limit behavior is documented or implemented for the task scope.
- [ ] Progress report and token usage log were updated.
- [ ] Handoff note was updated if pending work remains.
