# Cloudcell agent guide

Cloudcell runs untrusted agent code in kernel jails (AgentCell `sand`): namespaces, cgroup limits, Landlock, seccomp. Not Docker.

- Console: `https://cloudcell.dev`
- API: `https://api.cloudcell.dev`
- This file: `https://cloudcell.dev/docs/guide.md`
- Changelog: `https://cloudcell.dev/docs/changelog.md`
- Index: `https://cloudcell.dev/llms.txt`

Unauthenticated: `GET /health`, `GET /api/v1/ping`, `GET /api/v1/auth/status`. Everything else needs `Authorization: Bearer <token>`.

## Auth

Register (email first, then code + password):

```http
POST https://api.cloudcell.dev/api/v1/auth/send-code
Content-Type: application/json

{"email":"you@example.com"}
```

```http
POST https://api.cloudcell.dev/api/v1/auth/register
Content-Type: application/json

{"email":"you@example.com","code":"123456","password":"at-least-8-chars"}
```

Login returns a session token (`cc_sess_…`):

```http
POST https://api.cloudcell.dev/api/v1/auth/login
Content-Type: application/json

{"email":"you@example.com","password":"at-least-8-chars"}
```

Mint an API key for agents (`cc_live_…` plaintext is returned once):

```http
POST https://api.cloudcell.dev/api/v1/keys
Authorization: Bearer cc_sess_…
Content-Type: application/json

{"name":"agent"}
```

Use `Authorization: Bearer cc_live_…` on later calls. Sandboxes and keys are per-user; other users get 404.

## Snapshots

`GET /api/v1/snapshots`

| id | Use for |
| --- | --- |
| `python-3.12` | CPython 3.12, HTTPS, DNS |
| `rust` | rustc/cargo 1.98 + gcc linker |
| `base` | busybox-style shell only |
| `node-22` | currently the same as `base` (no Node packed yet) |

Default on create is `base`. Prefer `python-3.12` or `rust`.

## Sandboxes

Create (starts `sand serve` when the cell node has `CLOUDCELL_SAND`):

```http
POST https://api.cloudcell.dev/api/v1/sandboxes
Authorization: Bearer cc_live_…
Content-Type: application/json

{"snapshot":"python-3.12"}
```

Optional fields: `cpu` (default 1), `mem_bytes` (default 1073741824), `pids` (default 64), `egress` (string list; first entry is the allowlist, e.g. `["api.openai.com:443"]`). Empty `egress` means general outbound on veth. Metadata `169.254.0.0/16` is blocked.

Response `state` is `running` when the jail started, else `pending`.

Exec (argv is a JSON array, not a shell string):

```http
POST https://api.cloudcell.dev/api/v1/sandboxes/{id}/exec
Authorization: Bearer cc_live_…
Content-Type: application/json

{"argv":["python3","-c","print(1+1)"],"timeout_ms":30000}
```

Optional `stdin` string is written then EOF. Response: `{ "code": 0, "stdout": "…" }` (stderr is merged into stdout).

Delete (SIGTERM the jail):

```http
DELETE https://api.cloudcell.dev/api/v1/sandboxes/{id}
Authorization: Bearer cc_live_…
```

## Examples

Python HTTPS:

```json
{"argv":["python3","-c","import urllib.request; print(urllib.request.urlopen('https://example.com', timeout=15).status)"]}
```

Rust compile:

```json
{"argv":["/usr/bin/bash","-c","cat > a.rs && rustc a.rs -o a && ./a"],"stdin":"fn main() { println!(\"hi\"); }\n"}
```

## Limits (honest)

- One jail per sandbox. No cross-user recycle.
- Network is `--net veth`, never `--net host`.
- Restarting the API stops cells; rows become `stopped`. Recreate to run again.
- No PTY, no file upload/download API, no preview URL, no org/project tenancy.
- `node-22` is not a real Node image yet.
- Multiple `egress` entries: only the first is applied.
