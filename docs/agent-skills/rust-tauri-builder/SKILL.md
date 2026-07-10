# Rust Tauri Builder

## Purpose
Guide agents when working on the Rust/Tauri backend for AI Agent Trend Radar, including commands, services, models, app configuration, and desktop shell behavior.

## When to Use
Use this skill when adding or changing Rust code under `src-tauri/`, adjusting Tauri config, exposing backend commands to the Svelte frontend, or validating desktop app startup.

## Inputs
- Current task objective.
- Relevant files under `src-tauri/`.
- Current `AGENTS.md` rules.
- Any related progress or handoff notes.
- Expected command/API boundary between frontend and backend.

## Rules
- Read `AGENTS.md` before making changes.
- Keep changes small and focused.
- Do not introduce crates unless the task clearly requires them.
- Do not hardcode tokens, API keys, user IDs, database paths, or secrets.
- Prefer existing folders: `commands`, `services`, `models`, and `utils`.
- Keep placeholder code clearly named until real behavior is implemented.
- Validate with `cargo fmt --check` and `cargo check` when Rust changes are made.
- Update progress report, token usage log, and handoff note if work remains unfinished.

## Output Requirement
- Summarize Rust/Tauri files changed.
- State commands run and their results.
- Note any warnings that remain.
- Identify next backend task if the implementation is incomplete.

## Risk Note
Tauri config and command boundaries can break desktop startup even when frontend build passes. Avoid broad refactors and keep security-sensitive configuration outside source code.

## Completion Checklist
- [ ] `AGENTS.md` was followed.
- [ ] Changes are limited to the requested backend scope.
- [ ] No secret or credential was added.
- [ ] Rust formatting/checks were run or a reason was documented.
- [ ] Progress report and token usage log were updated.
- [ ] Handoff note was updated if pending work remains.
