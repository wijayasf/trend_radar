# Agent Working Rules

This project is a local-first desktop intelligence app. Agents must keep changes small, focused, and easy to hand off.

## Core Rules

- Make small focused changes per session.
- Avoid over-engineering and speculative abstractions.
- Do not hardcode tokens, API keys, user IDs, credentials, or secrets.
- Do not commit `.env`, local databases, generated logs, or secret files.
- Prefer local-first behavior and explicit configuration over hidden remote calls.
- Add dependencies only when they are needed for the current task.
- Keep placeholder code clearly marked until the related feature is implemented.

## Session Discipline

- Every session must update a progress report in `docs/agent-progress/`.
- Every session must record token start, token used, and token remaining when available.
- If token metrics are not available, write `Estimated` or `Unknown` and include the source.
- If a task is not finished, every session must create or update `docs/agent-handoff/latest-handoff.md`.
- Include validation performed, known risks, and the next recommended task.

## Security Boundaries

- Never paste real Threads API credentials into source code, docs, test fixtures, or logs.
- Use `.env.example` for placeholder configuration only.
- Keep local DuckDB files under `data/` and out of version control.
- Treat exported reports as user data.

## Engineering Preferences

- Rust/Tauri owns desktop shell, local services, storage access, and backend commands.
- Svelte/TypeScript owns the user interface.
- DuckDB is the intended local analytics store, but should be introduced only when schema work begins.
- Threads API integration should start behind a service interface before real request logic is added.
- Optional Ollama/local LLM classification belongs in a later phase behind a replaceable classifier interface.
