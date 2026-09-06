# Deploy: cloudcell.dev

DNS is on Cloudflare. Frontend is Pages. API is one GCP VM behind a Tunnel.

| Host | Target |
| --- | --- |
| `cloudcell.dev` / `www.cloudcell.dev` | Cloudflare Pages project `cloudcell` |
| `api.cloudcell.dev` | Cloudflare Tunnel → `127.0.0.1:8080` on the GCP VM |

Do not open `:80`/`:443` on the VM. Do not proxy `/api` through Pages (WebSocket and audit streams need the API origin).

## Pages (console)

Cloudflare dashboard → Workers & Pages → Create → Connect git repo.

- Project name: `cloudcell`
- Root directory: `admin`
- Build command: `npm ci && npm run build`
- Output directory: `dist`
- Environment variable: `VITE_API_BASE=https://api.cloudcell.dev`

Custom domains: `cloudcell.dev` and `www.cloudcell.dev`.

SPA fallback is `admin/public/_redirects` (`/* /index.html 200`).

Local:

```bash
cargo run
cd admin && npm install && npm run dev
```

Vite proxies `/api` and `/health` to `http://127.0.0.1:8080`. Leave `VITE_API_BASE` empty.

## GCP (API)

See [`deploy/gcp/README.md`](../deploy/gcp/README.md).

1. Provision Ubuntu 24.04 (x86_64 or aarch64).
2. Build `cloudcell` on the VM (or copy `target/release/cloudcell`).
3. Install `deploy/gcp/cloudcell.service` + `env.example`.
4. Create tunnel `cloudcell-api` and route `api.cloudcell.dev`.
5. CORS already allows `https://cloudcell.dev` and `https://www.cloudcell.dev`.

Health from the VM: `curl -sS http://127.0.0.1:8080/health`.

AgentCell (`sand`, `agentlsm`) is a later install on this same machine. Until then exec stays 501.
