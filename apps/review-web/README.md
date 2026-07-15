# AI Agent Trend Radar Review Web Demo

This is a lightweight public web demo for Threads App Review. It is intentionally separate from the main Tauri desktop app.

The demo stores the Threads access token on the server via environment variables and never exposes it to browser JavaScript.

## Local Setup

```bash
cd apps/review-web
npm install
cp .env.example .env
npm start
```

Open:

- `http://localhost:3000/health`
- `http://localhost:3000/`

## Environment Variables

```env
THREADS_ACCESS_TOKEN=
THREADS_USER_ID=
APP_ENV=review
PORT=3000
```

Do not commit `.env`.

## Render Deployment

- Root Directory: `apps/review-web`
- Build command: `npm install`
- Start command: `npm start`
- Environment variables:
  - `THREADS_ACCESS_TOKEN`
  - `THREADS_USER_ID`
  - `APP_ENV=review`

Before advanced access approval, Threads keyword search may only return posts owned by the authenticated tester account. After approval, public posts can be searched.
