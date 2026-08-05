# AI Agent Trend Radar

AI Agent Trend Radar is a local-first named entity radar for discovering AI agents, skills, MCP servers, frameworks, and tools, then preparing weekly Indonesia/global reports.

## Overview

AI Agent Trend Radar collects, organizes, and analyzes public posts only when the pipeline can extract a concrete named AI agent, coding tool, agent skill, MCP server, framework, registry, or related tool.

The application is designed as a local-first desktop app, using DuckDB for local analytics storage and Tauri for the desktop runtime.

## Key Objectives

- Track named AI Agent, tool, skill, MCP server, framework, and registry signals.
- Filter generic AI Agent discussion that contains no concrete entity name.
- Classify detected entities by category.
- Separate trend signals by Indonesia, global, or unknown region.
- Prepare weekly trend reports for research and internal decision support.
- Support local-first analysis without storing sensitive credentials in the repository.

## Tech Stack

- Rust
- Tauri
- DuckDB
- Svelte / TypeScript
- Threads API
- Experimental Apify fallback connector

## Main Capabilities

- Local DuckDB storage.
- Threads API integration.
- Named entity discovery crawler using seed keywords from `config/discovery_keywords.yml`.
- Experimental Apify Threads scraper fallback with an entity-first inclusion gate.
- Crawl diagnostics with run summary, seed-level status, bounded pagination, and single-seed testing.
- Safe environment-based configuration.
- Sample data import for local testing.
- Entity detection for AI Agent-related tools and skills.
- Candidate entity extraction for new or unknown tool names that need review.
- Candidate review workflow with durable approve/ignore decisions for unknown candidates.
- Region classification for Indonesia, global, and unknown signals.
- Sentiment and cost/boros signal classification.
- Weekly aggregation with trend score ranking.
- Markdown and CSV weekly report export.

## Entity Categories

The app supports categorization of detected entities into groups such as:

- Coding agent
- Coding assistant
- Generic agent framework
- Skill or mode
- MCP or connector
- Registry or discovery source
- App builder
- Unknown candidate
- Unknown

## Example Entities

Examples of supported entities include:

- Claude Code
- Cursor
- GitHub Copilot
- Codex CLI
- Cline
- OpenCode
- Caveman
- Ponytail
- Astryx
- ExplainX
- LangGraph
- CrewAI
- Replit Agent
- Bolt
- Lovable

## Environment Setup

Create a local `.env` file based on `.env.example`.

Required variable names are `THREADS_ACCESS_TOKEN`, `THREADS_USER_ID`, `APP_ENV`, and `DATABASE_PATH`.

Optional Apify fallback variables are documented in `.env.example`.

Do not commit `.env`.

## Local Development

Install dependencies:

```bash
npm install
```

Run frontend only:

```bash
npm run dev
```

Run the Tauri desktop app:

```bash
npx tauri dev
```

Build frontend:

```bash
npm run build
```

Check Rust backend:

```bash
cd src-tauri
cargo check
```

## MVP Workflow

Recommended local flow:

```text
Run AI Agent Discovery Crawl
→ Detect Agent Mentions
→ Review Unknown Candidates
→ Classify Regions
→ Classify Sentiments
→ Classify Cost Signals
→ Aggregate Weekly Metrics
→ Export Markdown/CSV Report
```

The manual Threads keyword collector remains available for debugging a single keyword.
Discovery crawl is the primary research flow. Search seeds may be broad, but a post is useful to the
radar only when entity detection extracts a concrete tool, agent, skill, named MCP server, framework,
or meaningful candidate name.
When Threads keyword search returns IDs only, the backend attempts a safe post detail fetch before
entity detection runs.

The desktop UI includes guided demo controls:

- `Run Full Sample Demo` imports sample posts and runs detection, classification, and weekly metrics.
- `Run Full Real Flow` runs discovery against Threads, then detection, classification, and weekly metrics.

Candidate review remains manual so new or unknown entities are not approved automatically.
Long-running actions show disabled buttons, loading labels, and a compact spinner so demo state is visible while the local pipeline runs.

Use `Clear Local Demo Data` to reset raw posts, mentions, crawl runs, and weekly metrics during demos. Durable candidate review decisions are preserved so approved/ignored candidate choices continue to apply.

## Apify Fallback Connector

The official Threads API remains the preferred connector. An experimental fallback connector is available for Apify actor `futurizerush/meta-threads-scraper` when review/demo work needs an alternate source.

The fallback stores the Apify token in backend environment variables only and labels raw rows with `source_type = apify_threads_scraper`. Before saving, it applies an entity-first gate using the same known aliases and strict unknown-candidate rules as entity detection. Generic AI Agent or MCP discussion is discarded when no concrete name can be extracted. `MCP` alone is context, not a rankable entity. Unknown names such as `Graphify` or `Headroom` enter Candidate Review as pending and do not affect weekly rankings until approved.

The actor requires at least 10 max posts. The backend adjusts smaller values to 10. Synchronous actor timeout defaults to 300 seconds and can be changed with `APIFY_RUN_TIMEOUT_SECONDS` (bounded to 30-900 seconds).

Do not use the Apify fallback in production without legal and compliance review.

## Review Web Demo

A lightweight web demo is available under `apps/review-web` for Threads App Review demonstration. It uses server-side environment variables and does not expose access tokens to the browser.

Render deployment:

- Root Directory: `apps/review-web`
- Build command: `npm install`
- Start command: `npm start`
- Environment variables: `THREADS_ACCESS_TOKEN`, `THREADS_USER_ID`, `APP_ENV=review`

## Project Structure

```text
.
├── apps/review-web/      # Public web demo for Threads App Review
├── src/                  # Svelte / TypeScript frontend
├── src-tauri/            # Rust / Tauri backend
├── config/               # Keywords, aliases, and scoring config
├── data/                 # Local runtime data, ignored by Git
├── docs/                 # Project documentation and agent progress notes
├── AGENTS.md             # AI agent working instructions
├── .env.example          # Environment variable template
└── README.md
```

## AI Agent Development Workflow

This project supports AI-assisted development workflows.

Agent guidance:

- Read `AGENTS.md` before working.
- Make small and focused changes.
- Avoid over-engineering.
- Do not hardcode credentials.
- Keep progress notes and handoff documentation updated.
- Track token usage when available.

## Security Notes

- Do not commit `.env`.
- Do not hardcode access tokens.
- Do not commit local DuckDB runtime files.
- Do not commit build artifacts.
- Keep API credentials in local environment configuration only.

## License

Internal research / prototype project.
