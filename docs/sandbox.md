# Sandbox contract (honest)

This file is the domain spec for `/api/v1/sandboxes`. Anything not listed as **implemented** is a skeleton.

## Implemented (control plane)

| Method | Path | Behavior |
| --- | --- | --- |
| GET | `/health` | `{ status: ok, service: cloudcell }` (no auth) |
| GET | `/api/v1/ping` | `{ message: pong }` (no auth) |
| GET/POST | `/api/v1/sandboxes` | SQLite records. Create is **`pending`** unless `CLOUDCELL_SAND` starts `sand serve` (**`running`**). |
| GET/DELETE | `/api/v1/sandboxes/{id}` | Record lookup / delete (SIGTERM the cell if live) |
| POST | `/api/v1/sandboxes/{id}/exec` | AgentCell serve protocol when a live cell exists; otherwise **501** / **409** |
| GET | `/api/v1/snapshots` | Static catalog (`base`, `python-3.12`, `node-22`), status **`declared`** |
| GET/POST | `/api/v1/keys` | Create returns plaintext **once**; store is SHA-256 only |
| DELETE | `/api/v1/keys/{id}` | Immediate invalidate |
| POST | `/api/v1/keys/{id}/rotate` | New plaintext once; old hash dropped |

Auth: `Authorization: Bearer cc_live_…` on every `/api/v1/*` route except `/api/v1/ping`. The first `POST /api/v1/keys` is allowed without a bearer while the `api_keys` table is empty (bootstrap). After that, missing/invalid tokens return 401.

Create defaults: `cpu=1`, `mem_bytes=1GiB`, `pids=64`, `snapshot=base`, `--net none`. Unknown snapshot → 400.

Local default DB is `sqlite:cloudcell.db`. Production: `CLOUDCELL_DATABASE_URL`.

Set `CLOUDCELL_SAND` to the AgentCell `sand` binary on a Linux node. The API **spawns `sand serve`** (subprocess, not `libagentcell`). Network is loopback-only. Restarting the API marks leftover `running` rows `stopped` (cells die with the process).

## Not implemented

- Packed snapshot erofs / `--rootfs` (catalog ids are declared only)
- `--net veth`, `--egress`, preview URLs, PTY, file upload
- `agentlsm` audit stream
- Org/project tenancy (one SQLite, all keys share one sandbox table)

## Invariants for the agent milestone

1. One `sand serve` per sandbox. Never reuse a jail across tenants.
2. Never `--net host`. Prefer `--net veth` + egress allowlist later. Block `169.254.169.254`.
3. Never accept AgentCell's default `memory.max` (60% RAM) on a shared node — create always passes `--mem`.
4. Gateway stays unprivileged. Root is only `agentlsm`.
5. Exec talks the serve protocol, not `Command::new("sand")` per request.
