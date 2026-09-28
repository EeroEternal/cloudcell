# Sandbox contract (honest)

This file is the domain spec for `/api/v1/sandboxes`. Anything not listed as **implemented** is a skeleton.

## Implemented (control plane)

| Method | Path | Behavior |
| --- | --- | --- |
| GET | `/health` | `{ status: ok, service: cloudcell }` (no auth) |
| GET | `/api/v1/ping` | `{ message: pong }` (no auth) |
| GET/POST | `/api/v1/sandboxes` | Per-user SQLite records. Create is **`pending`** unless `CLOUDCELL_SAND` starts `sand serve` (**`running`**). |
| GET/DELETE | `/api/v1/sandboxes/{id}` | Owner-only lookup / delete (SIGTERM the cell if live). Other users get 404. |
| POST | `/api/v1/sandboxes/{id}/exec` | AgentCell serve protocol when a live cell exists; otherwise **501** / **409** |
| GET | `/api/v1/sandboxes/{id}/stream` | WebSocket duplex stdio stream (AgentCell serve protocol v2 / ACP bridge) |
| GET | `/api/v1/sandboxes/{id}/acp` | Alias for `/stream` defaulting to `argv=["zene","acp"]` |
| GET | `/api/v1/snapshots` | Built-ins (`base`, `python-3.12`, `node-22`, `rust`, `go`) plus imported OCI snapshots; each has `status: declared`, `packed: true/false`, and imported ones carry `digest`/`image` |
| GET/POST | `/api/v1/keys` | Per-user. Create returns plaintext **once**; store is SHA-256 only |
| DELETE | `/api/v1/keys/{id}` | Immediate invalidate |
| POST | `/api/v1/keys/{id}/rotate` | New plaintext once; old hash dropped |

Auth: session (`cc_sess_…`) or API key (`cc_live_…`). Both resolve to a `user_id`; sandboxes and keys are scoped to that user. Register is email → send-code → password. Mail: `CF_EMAIL_*` or `MAIL_*`; otherwise the code is logged (dev).

Create defaults: `cpu=1`, `mem_bytes=1GiB`, `pids=64`, `snapshot=base`, `--net none`. Unknown snapshot → 400. When `CLOUDCELL_SAND` is set, a snapshot with no packed rootfs on this node is **not** silently downgraded to the host `/usr`: create fails fast with **400** and `error.reason = "snapshot_not_packed"`.

`egress` is a repeatable list of `HOST[:PORT]` (port defaults to 443), validated at create (bad host / IPv6 literal / bad port / empty → 400 `egress_invalid`). The cell's DNS under veth is systemd-resolved's upstream list (bound in by `sand`), and `sand` now sends those nameservers to `agentlsm`, which resolves each host against **them** and **refreshes on the record TTL** (capped at 60 s) — this is what makes CDN registries (`static.crates.io`, …) reachable despite within-resolver rotation. At boot/create the control plane probes `sand --capabilities` and refuses to run against a binary without `egress_multi` / `egress_refresh` / `egress_resolv` (400 `Config`). If the cell runtime cannot provision the requested egress it exits nonzero, and create returns **502** (`egress could not be provisioned`) instead of a silently network-less cell. `GET /snapshots` reports `packed` per snapshot so an agent can check readiness before creating.

`POST .../exec` default `timeout_ms` is **600000** (10 minutes) — long enough for a cold `clone → install → build`; pass `timeout_ms` to shorten or extend per request. On nonzero exit the reply also carries an additive `failure` object: `{ "reason": "oom" | "disk_full" | "network_denied" | "nonzero", "host"?, "detail": ... }` classified from the exit code (137 → OOM) and stderr.

### Warm caches (`caches`)

`caches: ["cargo", "pip", ...]` bind-mounts a stable **per-user** volume from `CLOUDCELL_CACHE_DIR/<user>/<name>` at the path each toolchain reads by default. Known names: `cargo` (`.cargo`), `rustup`, `pip` (`.cache/pip`), `npm`, `go`, `gradle`, `maven`. Cache volumes are **not** deleted when the sandbox is deleted; unknown names → 400.

### Disk cap (`disk_bytes`)

`disk_bytes` makes the workspace a RAM-backed tmpfs of that size instead of the host workdir bind, so a runaway build cannot fill the node disk. It is a hard kernel cap, but tmpfs pages count against `mem_bytes` — size both together. When unset, the workspace is the host bind and disk exhaustion is only detectable (→ `failure.reason = "disk_full"`), not prevented.

### Git auth (`git`)

`git: { token?, ssh_key? }` injects credentials at create and **never persists them**. A `token` (GitHub PAT) becomes cell env (`GITHUB_TOKEN`, `GH_TOKEN`, and a `GIT_CONFIG_*` `insteadOf` rule) delivered via a 0600 env file; `ssh_key` is copied into the cell's `/run/agentcell-git/id_ed25519` (0600, tmpfs) and wired through `GIT_SSH_COMMAND`. Host copies are removed as soon as the cell starts. `github.com:443` (token) / `github.com:22` (ssh) are appended to the egress allowlist automatically.

### OCI snapshots (content-addressed)

`deploy/gcp/import-oci.sh NAME docker://REF [LANGUAGE]` pulls an image (skopeo + umoci), unpacks it under `CLOUDCELL_ROOTFS_DIR/blobs/<digest>` and maps `NAME → digest` in `CLOUDCELL_ROOTFS_DIR/index.json`. `docker:REF` flattens a local image via `docker export`, and `dir:/PATH` imports an already-unpacked tree (hash-addressed) for air-gapped nodes or tests. The control plane reads that index, so imported snapshots appear in `GET /snapshots` (with `digest` + `image`) and can be requested by name. Re-importing the same image digest is a no-op. Pin an exact build by importing a digest ref (`docker://rust@sha256:…`) so "CI passed" is reproducible across nodes over time; `index.json` is the only mutable state and the rootfs tree is never edited in place. This replaces the host-extraction drift of `deploy/gcp/pack-rootfs.sh` (kept only as a legacy fallback); `go` is the R8 default (import `docker://docker.io/library/golang:1.23-bookworm`).

Local default DB is `sqlite:cloudcell.db`. Production: `CLOUDCELL_DATABASE_URL`.

Set `CLOUDCELL_SAND` to the AgentCell `sand` binary on a Linux node. The API **spawns `sand serve`** (subprocess, not `libagentcell`). A cell with a non-empty `egress` allowlist runs `--net veth`, and `sand` exits nonzero when that egress cannot be provisioned (create then returns **502**); a cell with **no** allowlist runs `--net none`, i.e. loopback-only — `--net veth` without `--egress` is *unrestricted* NAT egress in AgentCell, so it is never requested without a list. Restarting the API marks leftover `running` rows `stopped` (cells die with the process).

## Not implemented

- Single-file erofs/squashfs snapshot images (snapshots are unpacked directory trees under `blobs/<digest>`)
- Preview URLs, PTY, file upload
- `agentlsm` audit stream
- Org/project tenancy (per-user isolation only; no orgs yet)

## Invariants for the agent milestone

1. One `sand serve` per sandbox. Never reuse a jail across tenants.
2. Never `--net host`. Egress is an allowlist (`--net veth --egress …`) or loopback-only (`--net none`); never a veth without `--egress`, which is unrestricted NAT in AgentCell. Block `169.254.169.254`.
3. Never accept AgentCell's default `memory.max` (60% RAM) on a shared node — create always passes `--mem`.
4. Gateway stays unprivileged. Root is only `agentlsm`.
5. Exec talks the serve protocol, not `Command::new("sand")` per request.
