# GCP cell node (control plane + later agentcell)

One Linux VM is enough for P0. Cloudflare Tunnel publishes `api.cloudcell.dev`; the VM has no public HTTP ports.

## Machine

- Ubuntu 24.04 LTS, x86_64 or aarch64
- Unprivileged user namespaces (`kernel.unprivileged_userns_clone=1`)
- On Ubuntu 24.04 also relax or profile AppArmor userns (see AgentCell README)
- `clang`, `libbpf`, `libelf` only when you build `sand` / `agentlsm` on this host

## Control plane

```bash
sudo useradd --system --home /var/lib/cloudcell --create-home cloudcell
sudo install -m 0755 target/release/cloudcell /usr/local/bin/cloudcell
sudo install -d -o cloudcell -g cloudcell /var/lib/cloudcell /etc/cloudcell
sudo install -m 0640 deploy/gcp/env.example /etc/cloudcell/env
sudo install -m 0644 deploy/gcp/cloudcell.service /etc/systemd/system/cloudcell.service
sudo systemctl enable --now cloudcell
```

Health: `curl -sS http://127.0.0.1:8080/health`

## Tunnel

```bash
sudo cloudflared service install
# then fill deploy/gcp/cloudflared.yml and restart cloudflared
```

`agentlsm` / `sand` are **not** started by this unit. Wire them in a later milestone; until then `/api/v1/sandboxes/{id}/exec` returns 501.

## Snapshots

Import OCI images (preferred — content-addressed, no host drift):

```bash
sudo apt-get install -y skopeo umoci   # or use docker:REF / dir:PATH
sudo deploy/gcp/import-oci.sh rust docker://docker.io/library/rust:1.83-bookworm rust
sudo deploy/gcp/import-oci.sh go   docker://docker.io/library/golang:1.23-bookworm go
# no skopeo/umoci? flatten a local image with the docker daemon:
sudo deploy/gcp/import-oci.sh go docker:golang:1.23-bookworm go
# or register an already-unpacked tree (hash-addressed, air-gapped):
sudo deploy/gcp/import-oci.sh base dir:/var/lib/cloudcell/base-tree shell
curl -sS localhost:8080/api/v1/snapshots
```

Each image lands once under `/var/lib/cloudcell/snapshots/blobs/<digest>` and
`index.json` maps the name to that digest. Re-importing the same digest is a
no-op; pin `docker://image@sha256:…` for reproducible CI. `deploy/gcp/pack-rootfs.sh`
still works as a legacy host-extraction fallback but is not recommended.
