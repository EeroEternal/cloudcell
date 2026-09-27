#!/usr/bin/env bash
# Content-addressed OCI importer for cloudcell cell nodes.
#
#   import-oci.sh NAME docker://REF [LANGUAGE]     pull + unpack (skopeo/umoci)
#   import-oci.sh NAME docker:REF [LANGUAGE]        flatten via docker export
#   import-oci.sh NAME oci:/path/to/layout [LANG]  local OCI layout
#   import-oci.sh NAME dir:/path/to/rootfs [LANG]  already-unpacked tree
#
# The unpacked rootfs is stored once under
# $CLOUDCELL_ROOTFS_DIR/blobs/<digest> and NAME -> digest is recorded in
# index.json, which the control plane reads (see src/snapshot.rs).  Images
# are addressed by content, so re-importing the same image is a no-op and
# "CI passed" is reproducible across nodes and time.
#
#   sudo import-oci.sh rust docker://docker.io/library/rust:1.83-bookworm rust
#   sudo import-oci.sh go   docker://docker.io/library/golang:1.23-bookworm go
#
# Pin an exact build with a digest ref:
#   sudo import-oci.sh rust 'docker://rust@sha256:…' rust
set -euo pipefail

NAME=${1:?usage: import-oci.sh NAME docker://REF|oci:/PATH|dir:/PATH [LANGUAGE]}
REF=${2:?usage: import-oci.sh NAME docker://REF|oci:/PATH|dir:/PATH [LANGUAGE]}
LANG_=${3:-$NAME}
OUT="${CLOUDCELL_ROOTFS_DIR:-/var/lib/cloudcell/snapshots}"

if [ -n "${IMPORT_NO_SUDO:-}" ] || [ "$(id -u)" = 0 ]; then
    SUDO=()
else
    SUDO=(sudo)
fi

TMP=$(mktemp -d /tmp/oci-import.XXXXXX)
trap 'rm -rf "$TMP"' EXIT

DIGEST=""
SRC=""
LAYOUT=""

case "$REF" in
    dir:*)
        SRC=${REF#dir:}
        [ -d "$SRC" ] || { echo "no such dir: $SRC" >&2; exit 1; }
        echo "hashing local rootfs $SRC ..."
        DIGEST="sha256:$(tar -C "$SRC" --sort=name --mtime='UTC 1970-01-01' \
                        --owner=0 --group=0 --numeric-owner -cf - . \
                        | sha256sum | cut -d' ' -f1)"
        ;;
    oci:*)
        LAYOUT=${REF#oci:}
        [ -d "$LAYOUT" ] || { echo "no such OCI layout: $LAYOUT" >&2; exit 1; }
        ;;
    docker:*)
        DOCKER_REF=${REF#docker:}
        command -v docker >/dev/null || { echo "need docker (or use dir:/PATH)" >&2; exit 1; }
        CID=$(docker create "$DOCKER_REF")
        trap 'docker rm -f "$CID" >/dev/null 2>&1 || true; rm -rf "$TMP"' EXIT
        # prefer the immutable registry digest, else hash the flattened tree
        DIGEST=$(docker inspect --format '{{index .RepoDigests 0}}' "$DOCKER_REF" 2>/dev/null \
                 | sed -n 's/.*@\(sha256:[0-9a-f]*\)$/\1/p')
        echo "exporting $DOCKER_REF ..."
        mkdir -p "$TMP/rootfs"
        docker export "$CID" | tar -C "$TMP/rootfs" -xf -
        SRC="$TMP/rootfs"
        if [ -z "$DIGEST" ]; then
            echo "no RepoDigest on $DOCKER_REF; hashing exported tree ..."
            DIGEST="sha256:$(tar -C "$SRC" --sort=name --mtime='UTC 1970-01-01' \
                            --owner=0 --group=0 --numeric-owner -cf - . \
                            | sha256sum | cut -d' ' -f1)"
        fi
        ;;
    *)
        LAYOUT="$TMP/img"
        ;;
esac

if [ -z "$DIGEST" ]; then
    command -v skopeo >/dev/null || { echo "need skopeo (or use dir:/PATH)" >&2; exit 1; }
    command -v umoci  >/dev/null || { echo "need umoci (or use dir:/PATH)" >&2; exit 1; }
    ARCH=$(uname -m)
    case "$ARCH" in x86_64) ARCH=amd64 ;; aarch64) ARCH=arm64 ;; esac
    if [ ! -d "$LAYOUT" ]; then
        echo "fetching $REF ..."
        skopeo copy --override-os linux --override-arch "$ARCH" \
            "$REF" "oci:$LAYOUT:latest" >/dev/null
    fi
    DIGEST=$(python3 - "$LAYOUT" <<'PY'
import json, sys
idx = json.load(open(sys.argv[1] + "/index.json"))
print(idx["manifests"][0]["digest"])
PY
)
    [ -n "$DIGEST" ] || { echo "no manifest digest in $LAYOUT" >&2; exit 1; }
    echo "unpacking $DIGEST ..."
    umoci unpack --image "$LAYOUT:latest" "$TMP/bundle" >/dev/null
    SRC="$TMP/bundle/rootfs"
fi

DIR=$(echo "$DIGEST" | tr ':' '-')
BLOB="$OUT/blobs/$DIR"

"${SUDO[@]}" mkdir -p "$OUT/blobs"
if "${SUDO[@]}" test -d "$BLOB"; then
    echo "blob $DIGEST already present; skipping copy"
else
    "${SUDO[@]}" cp -a "$SRC" "$BLOB"
fi

"${SUDO[@]}" python3 - "$OUT/index.json" "$NAME" "$DIGEST" "$REF" "$LANG_" <<'PY'
import datetime, json, os, sys, tempfile
path, name, digest, image, language = sys.argv[1:6]
try:
    with open(path) as f:
        idx = json.load(f)
except (FileNotFoundError, ValueError):
    idx = {"version": 1, "snapshots": {}}
idx.setdefault("snapshots", {})[name] = {
    "digest": digest,
    "image": image,
    "language": language,
    "created_at": datetime.datetime.now(datetime.timezone.utc)
        .replace(microsecond=0).isoformat().replace("+00:00", "Z"),
}
d = os.path.dirname(os.path.abspath(path))
os.makedirs(d, exist_ok=True)
fd, tmp = tempfile.mkstemp(dir=d)
with os.fdopen(fd, "w") as f:
    json.dump(idx, f, indent=2, sort_keys=True)
    f.write("\n")
os.replace(tmp, path)
PY

"${SUDO[@]}" chown -R cloudcell:cloudcell "$BLOB" "$OUT/index.json" 2>/dev/null \
    || "${SUDO[@]}" chmod -R a+rX "$BLOB"

echo "snapshot '$NAME' -> $DIGEST  ($REF)"
echo "verify: curl -s http://127.0.0.1:8080/api/v1/snapshots"
