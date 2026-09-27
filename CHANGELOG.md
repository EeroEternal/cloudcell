# Changelog

All notable changes to the Cloudcell control plane.

## Unreleased

### Added

- `POST /sandboxes` validates the `egress` list (bad host / IPv6 literal /
  bad port / empty → 400 `egress_invalid`) and dedups it.
- Boot/first-create probe of `sand --capabilities`; create fails with 400
  if the binary lacks `egress_multi` / `egress_refresh` / `egress_resolv`.
- Create returns **502** (`egress could not be provisioned`) when the cell
  runtime cannot provision the requested egress, instead of a cell with no
  network.

## 0.2.0 — 2026-09-27

### Added

- **Warm caches** (`caches: ["cargo","pip",…]`): a stable per-user volume
  bind-mounted at each toolchain's default path. Cache volumes survive sandbox
  deletion. New config `CLOUDCELL_CACHE_DIR` (default `<data_dir>/cache`).
- **Git auth** (`git: { token?, ssh_key? }`): a PAT or SSH key injected at
  create via a 0600 env file / the cell's tmpfs, never stored in the DB.
  `github.com:443` (token) / `github.com:22` (key) are added to egress
  automatically.
- **`disk_bytes`**: RAM-backed, size-capped workspace (`sand --workdir-size`).
- **Content-addressed OCI snapshots**: `deploy/gcp/import-oci.sh` pulls /
  flattens / registers an image under `blobs/<digest>` and writes
  `index.json`; `GET /snapshots` reports `digest` and `image`. Built-in `go`
  snapshot added.
- `/exec` nonzero replies carry an additive
  `failure: { reason: "oom" | "disk_full" | "network_denied" | "nonzero",
  host?, detail }`.

### Changed

- `egress` is passed to the cell in full instead of only the first entry.
- `/exec` default `timeout_ms` raised from 30s to **600000** (10 minutes).
- Creating a sandbox whose snapshot is declared but not packed on this node
  returns **400** `error.reason = "snapshot_not_packed"` instead of silently
  falling back to the host `/usr`.
- `GET /snapshots` lists built-ins plus imported snapshots with `packed`.
- `deploy/gcp/pack-rootfs.sh` is legacy (host-extraction drift); prefer the
  OCI importer.

### Migration

- `006_workspace.sql` adds `sandboxes.caches_json` and `sandboxes.disk_bytes`.

## 0.1.0 — 2025

- Control plane: per-user sandboxes / API keys / sessions on SQLite, `sand
  serve` subprocess per sandbox, WebSocket stdio stream (`/stream`, `/acp`).
