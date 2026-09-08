#!/usr/bin/env bash
# Pack snapshot trees for sand --rootfs. Run on the cell node.
set -euo pipefail
AGENTCELL="${AGENTCELL:-$HOME/agentcell}"
OUT="${CLOUDCELL_ROOTFS_DIR:-/var/lib/cloudcell/snapshots}"
BUILD="$AGENTCELL/os/cell-root/build.sh"

[ -x "$BUILD" ] || { echo "missing $BUILD" >&2; exit 1; }
sudo mkdir -p "$OUT"

echo "packing base..."
sudo "$BUILD" --minimal "$OUT/base"

pack_python() {
    local dest=$1
    sudo rm -rf "$dest"
    sudo cp -a "$OUT/base" "$dest"
    local py
    py=$(type -P python3 || true)
    [ -n "$py" ] || return 0
    sudo mkdir -p "$dest/usr/bin"
    sudo cp -aL "$py" "$dest/usr/bin/python3"
    if [ -f /usr/bin/python3.12 ]; then sudo cp -a /usr/bin/python3.12 "$dest/usr/bin/"; fi
    ldd "$py" 2>/dev/null | awk '/=>/ {print $3} /^[[:space:]]*\// {print $1}' | while read -r so; do
        [ -f "$so" ] || continue
        sudo mkdir -p "$dest$(dirname "$so")"
        sudo cp -anL "$so" "$dest$so" 2>/dev/null || true
    done
    # stdlib (best-effort)
    for d in /usr/lib/python3 /usr/lib/python3.12 /usr/lib/python3.11 /usr/lib/python3.10; do
        [ -d "$d" ] || continue
        sudo mkdir -p "$dest$(dirname "$d")"
        sudo cp -a "$d" "$dest$d"
    done
    # OpenSSL used by ssl/https
    for lib in /lib/x86_64-linux-gnu/libssl.so.3 /lib/x86_64-linux-gnu/libcrypto.so.3; do
        [ -f "$lib" ] || continue
        sudo mkdir -p "$dest$(dirname "$lib")"
        sudo cp -anL "$lib" "$dest$lib" 2>/dev/null || true
    done
}

echo "packing python-3.12..."
pack_python "$OUT/python-3.12"

if type -P node >/dev/null; then
    echo "packing node-22 from host node..."
    sudo rm -rf "$OUT/node-22"
    sudo cp -a "$OUT/base" "$OUT/node-22"
    sudo cp -a "$(type -P node)" "$OUT/node-22/usr/bin/"
else
    echo "no node on host; node-22 snapshot = base"
    sudo rm -rf "$OUT/node-22"
    sudo cp -a "$OUT/base" "$OUT/node-22"
fi

copy_certs() {
    local dest=$1
    sudo mkdir -p "$dest/etc" "$dest/usr/lib"
    if [ -d /etc/ssl ]; then
        sudo cp -a /etc/ssl "$dest/etc/"
    fi
    if [ -e /usr/lib/ssl ]; then
        sudo cp -a /usr/lib/ssl "$dest/usr/lib/"
    fi
    # OpenSSL default verify path on Debian/Ubuntu
    if [ -f /etc/ssl/certs/ca-certificates.crt ]; then
        sudo mkdir -p "$dest/etc/ssl/certs"
        sudo cp -a /etc/ssl/certs/ca-certificates.crt "$dest/etc/ssl/certs/"
    fi
    # veth netns cannot use systemd-resolved at 127.0.0.53
    sudo mkdir -p "$dest/etc"
    printf 'nameserver 1.1.1.1\nnameserver 8.8.8.8\n' | sudo tee "$dest/etc/resolv.conf" >/dev/null
    if [ -f /etc/nsswitch.conf ]; then
        sudo cp -a /etc/nsswitch.conf "$dest/etc/"
    fi
}

copy_certs "$OUT/base"
copy_certs "$OUT/python-3.12"
copy_certs "$OUT/node-22"

sudo chown -R cloudcell:cloudcell "$OUT" 2>/dev/null || sudo chmod -R a+rX "$OUT"
echo "snapshots in $OUT:"
sudo du -sh "$OUT/base" "$OUT/python-3.12" "$OUT/node-22"
