#!/usr/bin/env bash
# Pack snapshot trees for sand --rootfs. Run on the cell node.
set -euo pipefail
AGENTCELL="${AGENTCELL:-$HOME/agentcell}"
OUT="${CLOUDCELL_ROOTFS_DIR:-/var/lib/cloudcell/snapshots}"
BUILD="$AGENTCELL/os/cell-root/build.sh"

[ -x "$BUILD" ] || { echo "missing $BUILD" >&2; exit 1; }
ONLY="${ONLY:-}"
sudo mkdir -p "$OUT"

if [ -z "$ONLY" ] || [ "$ONLY" = "base" ] || [ ! -d "$OUT/base" ]; then
    echo "packing base..."
    sudo "$BUILD" --minimal "$OUT/base"
fi

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

if [ -z "$ONLY" ] || [ "$ONLY" = "python-3.12" ]; then
echo "packing python-3.12..."
pack_python "$OUT/python-3.12"
fi

if [ -z "$ONLY" ] || [ "$ONLY" = "node-22" ]; then
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

copy_host() {
    local src=$1 dest=$2
    [ -e "$src" ] || return 0
    sudo mkdir -p "$dest$(dirname "$src")"
    sudo cp -a "$src" "$dest$src"
}

copy_bin() {
    local dest=$1 bin=$2
    local p
    p=$(type -P "$bin" || true)
    [ -n "$p" ] || return 0
    copy_host "$p" "$dest"
    ldd "$p" 2>/dev/null | awk '/=>/ {print $3} /^[[:space:]]*\// {print $1}' | while read -r so; do
        [ -f "$so" ] || continue
        copy_host "$so" "$dest"
    done
}

pack_rust() {
    local dest=$1
    command -v rustc >/dev/null || {
        echo "rustc not on PATH; skip rust snapshot" >&2
        return 0
    }
    echo "packing rust from $(rustc --print sysroot) ..."
    sudo rm -rf "$dest"
    sudo cp -a "$OUT/base" "$dest"
    local sysroot
    sysroot=$(rustc --print sysroot)
    sudo mkdir -p "$dest/usr/lib" "$dest/usr/bin"
    sudo cp -a "$sysroot" "$dest/usr/lib/rust"
    for b in rustc cargo rustdoc rustfmt clippy-driver; do
        if [ -x "$dest/usr/lib/rust/bin/$b" ]; then
            sudo ln -sfn /usr/lib/rust/bin/$b "$dest/usr/bin/$b"
        fi
    done
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq gcc binutils libc6-dev git >/dev/null
    for b in gcc cc as ld ar ranlib strip nm objcopy objdump git; do
        copy_bin "$dest" "$b"
    done
    local gccver
    gccver=$(gcc -dumpversion)
    copy_host "/usr/lib/gcc/x86_64-linux-gnu/$gccver" "$dest"
    for f in crt1.o crti.o crtn.o Scrt1.o libc.so libc_nonshared.a libm.so libpthread.so libdl.so librt.so; do
        copy_host "/usr/lib/x86_64-linux-gnu/$f" "$dest"
    done
    for lib in /lib/x86_64-linux-gnu/libssl.so.3 /lib/x86_64-linux-gnu/libcrypto.so.3 /lib/x86_64-linux-gnu/libgcc_s.so.1; do
        copy_host "$lib" "$dest"
    done
}

[ -d "$OUT/base" ] && copy_certs "$OUT/base"
[ -d "$OUT/python-3.12" ] && copy_certs "$OUT/python-3.12"
[ -d "$OUT/node-22" ] && copy_certs "$OUT/node-22"
if [ -z "$ONLY" ] || [ "$ONLY" = "rust" ]; then
    pack_rust "$OUT/rust"
    copy_certs "$OUT/rust"
fi

sudo chown -R cloudcell:cloudcell "$OUT" 2>/dev/null || sudo chmod -R a+rX "$OUT"
echo "snapshots in $OUT:"
sudo du -sh "$OUT/base" "$OUT/python-3.12" "$OUT/node-22" "$OUT/rust" 2>/dev/null || true
