# Cloudcell agent guide

Cloudcell runs untrusted agent code in kernel jails (AgentCell `sand`): namespaces, cgroup limits, Landlock, seccomp. Not Docker.

- Console: `https://cloudcell.dev`
- API: `https://api.cloudcell.dev`
- This file: `https://cloudcell.dev/docs/guide.md`
- Changelog: `https://cloudcell.dev/docs/changelog.md`
- Index: `https://cloudcell.dev/llms.txt`

**Do not register yourself.** Registration emails a 6-digit code a human must read. Ask the operator for a `cc_live_…` API key (created once in the console). Store it and use it as `Authorization: Bearer`.

Unauthenticated: `GET /health`, `GET /api/v1/ping`, `GET /api/v1/auth/status`. Everything else needs a bearer token.

## Minimal loop

```bash
export CC_API=https://api.cloudcell.dev
export CC_KEY=cc_live_…   # from the human

curl -sS "$CC_API/api/v1/sandboxes" \
  -H "authorization: Bearer $CC_KEY"

SID=$(curl -sS -X POST "$CC_API/api/v1/sandboxes" \
  -H "authorization: Bearer $CC_KEY" \
  -H "content-type: application/json" \
  -d '{"snapshot":"python-3.12"}' | jq -r .id)

curl -sS "$CC_API/api/v1/sandboxes/$SID" \
  -H "authorization: Bearer $CC_KEY"
# wait until "state":"running"

curl -sS -X POST "$CC_API/api/v1/sandboxes/$SID/exec" \
  -H "authorization: Bearer $CC_KEY" \
  -H "content-type: application/json" \
  -d '{"argv":["python3","-c","print(1+1)"]}'

curl -sS -X DELETE "$CC_API/api/v1/sandboxes/$SID" \
  -H "authorization: Bearer $CC_KEY"
```

Reuse a `running` sandbox for several execs. If `state` is `stopped` or exec returns 409/501, **create a new sandbox** — do not retry exec on the old id.

## Auth (humans)

Register is email → send-code → password. Login returns `cc_sess_…`. Then:

```http
POST /api/v1/keys
Authorization: Bearer cc_sess_…
Content-Type: application/json

{"name":"agent"}
```

Plaintext `cc_live_…` is returned **once**. `GET /api/v1/keys`, `DELETE /api/v1/keys/{id}`, `POST /api/v1/keys/{id}/rotate` exist. Sandboxes and keys are per-user; other users get 404.

## Snapshots

`GET /api/v1/snapshots`

| id | Use for |
| --- | --- |
| `python-3.12` | CPython 3.12, HTTPS, DNS |
| `rust` | rustc/cargo 1.98 + gcc linker |
| `base` | shell only (`bash`, `ls`, …) |
| `node-22` | same as `base` (Node is not packed) |

Default on create is `base`. Prefer `python-3.12` or `rust`.

## Sandboxes

```http
GET  /api/v1/sandboxes
POST /api/v1/sandboxes
GET  /api/v1/sandboxes/{id}
DELETE /api/v1/sandboxes/{id}
POST /api/v1/sandboxes/{id}/exec
```

Create:

```http
POST /api/v1/sandboxes
Authorization: Bearer cc_live_…
Content-Type: application/json

{"snapshot":"python-3.12"}
```

Optional: `cpu` (default 1), `mem_bytes` (default 1073741824), `pids` (default 64), `egress` (string list; **only the first** entry is the allowlist, e.g. `["api.openai.com:443"]`). Empty `egress` means general outbound on veth. `169.254.0.0/16` is blocked.

Response fields: `id`, `snapshot`, `state` (`pending` | `running` | `stopped`), `cpu`, `mem_bytes`, `pids`, `egress`, `created_at`. `state` is `running` when the jail started.

### Exec

`argv` is a JSON **array**, not a shell string. Do not send `"python3 -c '…'"` as one string.

```http
POST /api/v1/sandboxes/{id}/exec
Authorization: Bearer cc_live_…
Content-Type: application/json

{"argv":["python3","-c","print(1+1)"],"timeout_ms":30000}
```

- Optional `stdin` is written then EOF.
- Default `timeout_ms` is **30000**. Timeout → **400** `{ "error": { "message": "exec timed out", "code": 400 } }`.
- Response: `{ "code": 0, "stdout": "…" }`. **stderr is merged into stdout.** Process exit is `code` (not HTTP).
- CWD is `/home/agent` (writable). The snapshot rootfs is mostly read-only; install packages into the workdir, not `/usr`.
- Files in `/home/agent` last for the life of that sandbox only. API restart kills the jail; the row becomes `stopped`.

Python HTTPS:

```json
{"argv":["python3","-c","import urllib.request; print(urllib.request.urlopen('https://example.com', timeout=15).status)"]}
```

Rust compile (workdir is writable):

```json
{"argv":["/usr/bin/bash","-c","cat > a.rs && rustc a.rs -o a && ./a"],"stdin":"fn main() { println!(\"hi\"); }\n"}
```

## Errors

JSON body is always `{ "error": { "message": "…", "code": <http status> } }`.

| HTTP | When |
| --- | --- |
| 400 | Bad snapshot, empty `argv`, exec timed out |
| 401 | Missing/invalid bearer |
| 404 | Unknown id **or** another user's sandbox (do not treat as "exists") |
| 409 | `state` is `stopped` — create a new sandbox |
| 501 | No live cell: `pending` (node has no `sand`) or `running` row but process lost after API restart — create a new sandbox |

## Limits (honest)

- One jail per sandbox. No cross-user recycle.
- Network is `--net veth`, never `--net host`.
- Restarting the API stops cells. Recreate to run again.
- No PTY, no file upload/download API, no preview URL, no org/project tenancy, no SDK (raw HTTP only).
- `node-22` is not a real Node image yet.
- Multiple `egress` entries: only the first is applied.
