# Govrix Scout Dashboard

A React + Vite dashboard for [Govrix Scout](https://github.com/manaspros/govrix-scout) — open-source AI agent observability.

## Pages

| Route | Description |
|---|---|
| `/overview` | Stat cards (agents, events, cost, latency), requests chart, recent events |
| `/agents` | Filterable agents table with expandable detail rows |
| `/events` | Events table with time range filter, auto-refresh, and slide-out detail drawer |
| `/costs` | Daily cost chart, breakdown by agent / model / protocol |
| `/reports` | Generate and download compliance reports |
| `/settings` | Connection status, Scout version, proxy config display |

## Tech Stack

- **React 18** + **TypeScript**
- **Vite 5** (dev server + production build)
- **TanStack Query v5** (data fetching with auto-refresh)
- **Tailwind CSS v3** (dark slate theme)
- **Recharts** (area and bar charts)
- **React Router v6** (client-side routing)
- **Lucide React** (icons)

## Development

```bash
pnpm install
pnpm dev        # starts at http://localhost:3000
```

The Vite dev server proxies `/api`, `/health`, and `/metrics` to the Scout API server at `http://localhost:8080`.

Make sure Scout is running before starting the dashboard:

```bash
# In the scout repo root:
cargo run --bin govrix-scout-server
```

## Production Build

```bash
pnpm build      # outputs to dist/
pnpm preview    # preview the production build locally
```

## API Endpoints Used

| Endpoint | Purpose |
|---|---|
| `GET /api/v1/agents` | Agent list + stats |
| `GET /api/v1/events` | Event stream with filtering |
| `GET /api/v1/costs/summary` | Cost totals |
| `GET /api/v1/costs/breakdown` | Cost by agent / model / protocol |
| `GET /api/v1/reports/types` | Available report types |
| `GET /api/v1/reports` | Generated report list |
| `POST /api/v1/reports/generate` | Trigger report generation |
| `GET /api/v1/config` | Server configuration |
| `GET /health` | Server health + version |

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `VITE_API_URL` | `""` (same-origin) | Override API base URL for production |
