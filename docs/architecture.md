# Architecture & Domain Boundaries

Cloudcell is the multi-tenant control plane for [AgentCell](https://github.com/EeroEternal/agentcell). Isolation is not reimplemented here.

## 1. Topology

```text
browser  →  https://cloudcell.dev          Cloudflare Pages (admin/)
SDK      →  https://api.cloudcell.dev      Cloudflare Tunnel → GCP VM :8080
                 │
                 ▼
          cloudcell (this crate, unprivileged)
                 │  later: unix socket / gRPC
                 ▼
          sand serve  +  agentlsm (Linux cell node)
```

| Plane | Process | Privilege | Lives |
| --- | --- | --- | --- |
| Console | Vite / Pages | none | Cloudflare |
| Control plane | `cloudcell` binary | unprivileged | GCP VM |
| Cell runtime | `sand serve` | unprivileged | same or future nodes |
| Kernel policy | `agentlsm` | root, one per node | cell node |

The control plane **must not** link `libagentcell` / the `agentcell` crate to spawn jails. That ABI is one-shot, not thread-safe, and caps 16 live cells per process. Long-lived sandboxes are `sand serve` subprocesses driven over the unix-socket exec protocol.

## 2. Layering & Invocation Rules

1. **Transport (`src/server.rs`)**: route mounting, CORS, trace. No database queries, no jail lifecycle.
2. **Domain (`src/sandbox.rs`, `src/api_key.rs`, `src/snapshot.rs`)**: entities and state transitions. In-memory until sqlite migrations land.
3. **Storage**: `sqlx` + `migrations/NNN_*.sql` when persistence is added. Do not ad-hoc `std::fs` schemas.
4. **Cell agent (not in this crate yet)**: owns `sand`, cgroup accounting, and audit tails.

## 3. Plugin-First Principle

Auth decoration, egress policy, vendor protocol adaptors, and data masking are middleware/plugins. They are not hardcoded into sandbox create/exec.

## 4. What is not implemented

See [`sandbox.md`](sandbox.md). Do not describe pending/exec-501 paths as a running cell fleet.
