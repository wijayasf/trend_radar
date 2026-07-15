# AI Agent Trend Radar

AI Agent Trend Radar is a local-first desktop intelligence app for tracking AI Agent trend signals and preparing weekly Indonesia/global reports.

## Overview

AI Agent Trend Radar helps collect, organize, and analyze public trend signals related to AI agents, AI coding tools, agent skills, MCP, registries, and related developer workflows.

The application is designed as a local-first desktop app, using DuckDB for local analytics storage and Tauri for the desktop runtime.

## Key Objectives

- Track AI Agent discussion signals.
- Detect AI Agent, tool, skill, MCP, and registry mentions.
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

## Main Capabilities

- Local DuckDB storage.
- Threads API integration.
- AI Agent discovery crawler using broad seed keywords from `config/discovery_keywords.yml`.
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
- MCP
- LangGraph
- CrewAI
- Replit Agent
- Bolt
- Lovable

## Environment Setup

Create a local `.env` file based on `.env.example`.

```env
THREADS_ACCESS_TOKEN=
THREADS_USER_ID=
APP_ENV=local
DATABASE_PATH=./data/app.duckdb
```

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
Discovery crawl is the primary research flow because it searches broad AI Agent topics first,
then entity detection extracts specific tools, agents, skills, MCP terms, and candidate names.
When Threads keyword search returns IDs only, the backend attempts a safe post detail fetch before
entity detection runs.

The desktop UI includes guided demo controls:

- `Run Full Sample Demo` imports sample posts and runs detection, classification, and weekly metrics.
- `Run Full Real Flow` runs discovery against Threads, then detection, classification, and weekly metrics.

Candidate review remains manual so new or unknown entities are not approved automatically.
Long-running actions show disabled buttons, loading labels, and a compact spinner so demo state is visible while the local pipeline runs.

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
