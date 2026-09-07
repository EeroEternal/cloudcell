# Cloudcell

Sandbox-as-a-service control plane on [AgentCell](https://github.com/EeroEternal/agentcell).

- Console: `https://cloudcell.dev` (Cloudflare Pages, `admin/`)
- API: `https://api.cloudcell.dev` (GCP VM + Cloudflare Tunnel)

Sandboxes and API keys persist in SQLite. First `POST /api/v1/keys` bootstraps auth. On a Linux node set `CLOUDCELL_SAND` to the AgentCell `sand` binary to spawn `sand serve` and exec over its unix socket; otherwise exec returns 501. See [`docs/sandbox.md`](docs/sandbox.md).

## Quick start

```bash
cargo test
cargo run
# other terminal
cd admin && npm install && npm run dev
```

Open `http://127.0.0.1:5173`. The Vite dev server proxies `/api` to `:8080`.

## Layout

| Path | Role |
| --- | --- |
| `src/` | Axum control plane |
| `admin/` | Operator console (kit from console-kit) |
| `docs/design.md` | UI judgment; vocabulary is `admin/src/components/ui` |
| `docs/architecture.md` | Planes and crate rules |
| `docs/sandbox.md` | Honest API contract |
| `docs/deploy.md` | Pages + GCP + Tunnel |
| `AGENTS.md` | Agent entry (do not inflate) |

## Deploy

[`docs/deploy.md`](docs/deploy.md) and [`deploy/gcp/`](deploy/gcp/).

---
*Created with [OpenHub](https://openhub.run) - Open Agent & Git Platform*
