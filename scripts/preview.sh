#!/bin/bash
# scripts/preview.sh — Boot Alpine via QEMU with SPICE for a Remmina live preview.
#
# Starts the existing Alpine ISO in QEMU and exposes a SPICE display server so
# you can connect to a live OS from this host using Remmina, virt-viewer, or
# remote-viewer. Defaults are deliberately local-only.
#
# Defaults (override with environment variables):
#   SPICE_PORT      5900           Port the SPICE display listens on
#   SPICE_BIND      127.0.0.1      Bind address (loopback; use 0.0.0.0 for LAN)
#   MEMORY          2G             RAM for the guest
#   CORES           2              vCPU count
#   SPICE_PASSWORD  "" (empty)     If set, SPICE requires this password
#
# Network:
#   * SSH from the host:  ssh -p 2222 root@127.0.0.1   (Alpine ISO has no default password)
#
# Requirements:
#   apt install qemu-system-x86   # provides /bin/qemu-system-x86_64
#   Remmina on the client (with the SPICE plugin) — or remote-viewer / virt-viewer.
#
# Connection examples from this host:
#   Remmina      : File -> New Connection Profile, Protocol=SPICE,
#                   Server=127.0.0.1, Port=5900
#   Quick Connect: spice://127.0.0.1:5900
#   remote-viewer: spice://127.0.0.1:5900

set -euo pipefail

# Resolve script-relative paths even when invoked from elsewhere.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ASSETS_DIR="$PROJECT_DIR/assets"
ISO="$ASSETS_DIR/alpine.iso"

# ---- Configurable knobs (env overrides) ----
SPICE_PORT="${SPICE_PORT:-5900}"
SPICE_BIND="${SPICE_BIND:-127.0.0.1}"
MEMORY="${MEMORY:-2G}"
CORES="${CORES:-2}"
SPICE_PASSWORD="${SPICE_PASSWORD:-}"

# ---- Pre-flight checks ----
if [[ ! -f "$ISO" ]]; then
    cat >&2 <<EOF
ERROR: Alpine ISO not found at $ISO.

The repo ships assets/alpine.iso, which is what this script boots.
Other ISOs in assets/ are broken stubs:
EOF
    ls -la "$ASSETS_DIR" 2>/dev/null | sed 's/^/    /' >&2 || true
    cat >&2 <<EOF

To fetch a working Alpine ISO:
  curl -L -o "$ISO" \\
    https://dl-cdn.alpinelinux.org/alpine/v3.20/releases/x86_64/alpine-standard-3.20.0-x86_64.iso
EOF
    exit 1
fi

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo "ERROR: qemu-system-x86_64 not found. Install with: sudo apt install qemu-system-x86" >&2
    exit 1
fi

# ---- Compose SPICE option string ----
SPICE_ARGS="port=${SPICE_PORT},addr=${SPICE_BIND}"
if [[ -n "$SPICE_PASSWORD" ]]; then
    SPICE_ARGS+=",password=${SPICE_PASSWORD}"
else
    # disable-ticketing = no password required. Safe because we bind 127.0.0.1.
    SPICE_ARGS+=",disable-ticketing=on"
fi

# ---- Banner + Remmina instructions ----
iso_size=$(du -h "$ISO" 2>/dev/null | cut -f1)
auth_label=$([ -n "$SPICE_PASSWORD" ] && echo "password required" || echo "none (loopback-only)")

cat <<EOF
============================================================
  OblivionOS Live Preview  —  QEMU + SPICE
============================================================
  ISO         : $ISO ($iso_size)
  SPICE listen: ${SPICE_BIND}:${SPICE_PORT}
  Memory      : $MEMORY   Cores: $CORES
  Auth        : $auth_label
  SSH forward : host:2222 -> guest:22
------------------------------------------------------------
  Connect from THIS host with one of:
    Remmina       : File -> New Connection Profile,
                    Protocol=SPICE, Server=${SPICE_BIND}, Port=${SPICE_PORT}
    Quick Connect : spice://${SPICE_BIND}:${SPICE_PORT}
    remote-viewer : spice://${SPICE_BIND}:${SPICE_PORT}
    virt-viewer   : (open the URL above)
EOF
if [[ -n "$SPICE_PASSWORD" ]]; then
    cat <<EOF
  Password      : $SPICE_PASSWORD
EOF
fi
cat <<EOF
============================================================
EOF

# ---- Launch QEMU in the background ----
qemu-system-x86_64 \
    -m "$MEMORY" \
    -smp "$CORES" \
    -machine accel=kvm:tcg \
    -vga qxl \
    -display none \
    -spice "$SPICE_ARGS" \
    -cdrom "$ISO" \
    -netdev user,id=net0,hostfwd=tcp::2222-:22 \
    -device virtio-net-pci,netdev=net0 \
    -name "OblivionOS-Preview" \
    &
QEMU_PID=$!

# ---- Cleanup on exit / signal ----
# `QEMU_CLEANED` makes cleanup idempotent — when bash fires both INT
# (Ctrl-C) and EXIT for the same shell, we only actually kill QEMU once.
# `set -u` is on, so the variable is initialised explicitly below.
QEMU_CLEANED=0
cleanup() {
    local rc=$?
    if [[ $QEMU_CLEANED -eq 1 ]]; then
        # Already reaped on a prior signal — just propagate the exit code.
        return $rc
    fi
    QEMU_CLEANED=1

    echo "" >&2
    echo "[preview] stopping QEMU (pid ${QEMU_PID:-unset})..." >&2
    if [[ -n "${QEMU_PID:-}" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
        kill -TERM "$QEMU_PID" 2>/dev/null || true
        # Give QEMU up to 5s to exit cleanly on its own.
        for _ in 1 2 3 4 5 6 7 8 9 10; do
            kill -0 "$QEMU_PID" 2>/dev/null || break
            sleep 0.5
        done
        if kill -0 "$QEMU_PID" 2>/dev/null; then
            echo "[preview] QEMU did not respond to SIGTERM; sending SIGKILL" >&2
            kill -KILL "$QEMU_PID" 2>/dev/null || true
        fi
    fi
    # `wait` may return non-zero when the child was signal-killed; guard
    # against `set -e` triggering an abort right at the end.
    wait "$QEMU_PID" 2>/dev/null || true
    return $rc
}
trap cleanup EXIT INT TERM

# ---- Wait for the SPICE listener to bind ----
echo "[preview] waiting for SPICE listener on ${SPICE_BIND}:${SPICE_PORT}..." >&2
listener_up=0
for _ in $(seq 1 20); do
    # bash's /dev/tcp pseudo-fs gives us a dependency-free TCP probe.
    if bash -c "exec 3<>/dev/tcp/${SPICE_BIND}/${SPICE_PORT}" >/dev/null 2>&1; then
        listener_up=1
        break
    fi
    sleep 0.5
done
if [[ $listener_up -eq 1 ]]; then
    echo "[preview] SPICE listener is up — connect from Remmina now." >&2
else
    cat >&2 <<EOF
[preview] WARNING: SPICE listener did NOT appear within ~10s.
           QEMU may have failed to start; check the output above.
EOF
fi

# Foreground-wait so Ctrl-C cleans up via the trap.
wait "$QEMU_PID"
