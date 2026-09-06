# Sandbox contract (honest)

This file is the domain spec for `/api/v1/sandboxes`. Anything not listed as **implemented** is a skeleton.

## Implemented (control plane)

| Method | Path | Behavior |
| --- | --- | --- |
| GET | `/health` | `{ status: ok, service: cloudcell }` |
| GET | `/api/v1/ping` | `{ message: pong }` |
| GET/POST | `/api/v1/sandboxes` | In-memory records. Create state is **`pending`**. |
| GET/DELETE | `/api/v1/sandboxes/{id}` | Record lookup / delete |
| POST | `/api/v1/sandboxes/{id}/exec` | **501** until a Linux cell node runs agentcell |
| GET | `/api/v1/snapshots` | Static catalog (`base`, `python-3.12`, `node-22`), status **`declared`** |
| GET/POST | `/api/v1/keys` | Create returns plaintext **once**; store is SHA-256 only |
| DELETE | `/api/v1/keys/{id}` | Immediate invalidate |
| POST | `/api/v1/keys/{id}/rotate` | New plaintext once; old hash dropped |

Create defaults: `cpu=1`, `mem_bytes=1GiB`, `pids=64`, `snapshot=base`. Unknown snapshot → 400.

## Not implemented

- Starting `sand serve`, unix-socket exec, PTY, file upload, preview URLs
- Packing snapshot erofs / pulling OCI
- `agentlsm` audit stream
- sqlite persistence (records die with the process)
- Authn on the HTTP API (keys are stored, not yet required on requests)

## Invariants for the agent milestone

1. One `sand serve` per sandbox. Never reuse a jail across tenants.
2. Never `--net host`. Prefer `--net veth` + egress allowlist. Block `169.254.169.254`.
3. Never accept AgentCell's default `memory.max` (60% RAM) on a shared node.
4. Gateway stays unprivileged. Root is only `agentlsm`.
5. Exec talks the serve protocol (`examples/client.py` in agentcell), not `Command::new("sand")` per request.
