# Cloudcell changelog

Dates are UTC. This is the product log for agents and operators, not git history.

## 2026-09-08

- Snapshot `rust`: rustc/cargo 1.98.1, gcc, glibc, public DNS. `rustc` hello-world and `cargo --version` work.
- Packer copies CA certs, OpenSSL, `_ssl`, and `1.1.1.1`/`8.8.8.8` into rootfs so Python HTTPS verifies.

## 2026-09-07

- Jails use packed `--rootfs` under `/var/lib/cloudcell/snapshots/<id>` when the tree exists.
- `--net veth` plus `agentlsm serve`. Drop `10.200.0.0/16` → `169.254.0.0/16` (GCP metadata).
- Landlock `create_ruleset` retries attr sizes 8/16/24 so GCP kernel 7.0 no longer `EINVAL`.
- Per-user isolation: sandbox and API-key rows are scoped to `user_id`. Cross-user get/exec/delete → 404.

## 2026-09-06

- Email register (send-code → password) and login. Session `cc_sess_*`, API keys `cc_live_*`.
- Console at `https://cloudcell.dev` (Pages). API at `https://api.cloudcell.dev`.
- `sand serve` subprocess from the control plane. Never links `libagentcell`. Explicit `--mem/--cpu/--pids`.

## Known gaps

- `node-22` image is `base`.
- File upload/download, PTY, preview URLs.
- Persist cells across API restart.
- erofs single-file snapshots (directory trees work).
- Org/project tenancy.
