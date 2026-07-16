#!/bin/bash
# scripts/preview-rdp.sh — Boot OblivionOS as a true RDP target.
#
# Two-phase model:
#
#   ./scripts/preview-rdp.sh            # auto: install if qcow2 missing,
#                                       #       otherwise boot RDP
#   ./scripts/preview-rdp.sh --install  # one-time install: writes
#                                       #   assets/oblivionos-rdp.qcow2
#                                       #   with Alpine + xfce + xrdp.
#                                       #   Drives setup-alpine + apk via
#                                       #   expect on a serial console.
#   ./scripts/preview-rdp.sh --boot     # every run: boot the qcow2,
#                                       #   forward RDP_PORT -> guest:3389,
#                                       #   wait for xrdp, then block.
#
# Connection from the same host (default loopback-only on 3390):
#   Remmina -> File -> New Connection Profile ->
#     Protocol: RDP
#     Server:   127.0.0.1
#     Username: root
#     Port:     3390
#
# IMPORTANT: the default RDP_PORT is 3390, not 3389, because many desktops
# (including GNOME via gnome-remote-desktop-daemon on Debian/Ubuntu/Fedora)
# already bind 3389 for their own screen-sharing service. Set RDP_PORT=3389
# if you've disabled that service and want the canonical port.
#
# Requirements:
#   apt install qemu-system-x86 qemu-utils expect
#   Remmina (or any RDP client) on this host.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ASSETS_DIR="$PROJECT_DIR/assets"
ISO="$ASSETS_DIR/alpine.iso"
QCOW2="$ASSETS_DIR/oblivionos-rdp.qcow2"
SHARE_DIR="$ASSETS_DIR/share"
ANSWERS_FILE="$SCRIPT_DIR/alpine-answers.conf"
POST_INSTALL="$SCRIPT_DIR/alpine-rdp-post.sh"
EXPECT_SCRIPT="$SCRIPT_DIR/alpine-rdp-install.exp"

MEMORY="${MEMORY:-2G}"
CORES="${CORES:-2}"
RDP_BIND="${RDP_BIND:-127.0.0.1}"
RDP_PORT="${RDP_PORT:-3390}"
QCOW2_SIZE="${QCOW2_SIZE:-6G}"

# ---- RDP_BIND validation: dotted-quad IPv4 only.
# We accept 127.0.0.1, 0.0.0.0 (LAN), 192.168.x.y, etc., but reject
# hostnames ('localhost'), IPv6 (':1'), and empty. This prevents QEMU
# from returning the cryptic
#   "Could not set up host forwarding rule 'tcp:<hostfwd>'"
# when the user passes a typo.
if ! [[ "$RDP_BIND" =~ ^[0-9]+(\.[0-9]+){3}$ ]]; then
    cat >&2 <<EOF
ERROR: RDP_BIND='$RDP_BIND' is not a literal IPv4 dotted-quad address.

  Examples of valid values: 127.0.0.1, 0.0.0.0, 192.168.1.10
  Examples of invalid values: localhost, ::1, 10.0.0 (single octet omitted)

  Set RDP_BIND via env var or pass it on the command line.
EOF
    exit 1
fi

# ---- Pre-flight (always run) ---------------------------------------
mkdir -p "$ASSETS_DIR"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo "ERROR: qemu-system-x86_64 not found. Run: sudo apt install qemu-system-x86" >&2
    exit 1
fi

# ---- Mode selection -------------------------------------------------
mode="${1:-auto}"
case "$mode" in
    auto)
        if [[ ! -f "$QCOW2" ]]; then
            MODE_INSTALL=1
            MODE_BOOT=0
        else
            MODE_INSTALL=0
            MODE_BOOT=1
        fi
        ;;
    --install)
        MODE_INSTALL=1
        MODE_BOOT=0
        ;;
    --boot)
        MODE_INSTALL=0
        MODE_BOOT=1
        ;;
    *)
        cat >&2 <<EOF
Usage: $0 {auto|--install|--boot}

  auto      install if qcow2 missing, otherwise boot (default)
  --install one-time install via expect + setup-alpine + apk
  --boot    launch qcow2 with RDP on 127.0.0.1:3390 (default)

Env overrides: RDP_BIND, RDP_PORT, MEMORY, CORES, QCOW2_SIZE.
EOF
        exit 1
        ;;
esac

# ---- Install phase --------------------------------------------------
run_install_phase() {
    if ! command -v expect >/dev/null 2>&1; then
        echo "[install] expect not found; installing via apt..." >&2
        if command -v sudo >/dev/null 2>&1; then
            sudo apt install -y expect >&2 || {
                echo "ERROR: 'sudo apt install expect' failed. Install it manually." >&2
                exit 1
            }
        else
            apt install -y expect >&2 || {
                echo "ERROR: 'apt install expect' failed and sudo is missing." >&2
                exit 1
            }
        fi
        command -v expect >/dev/null 2>&1 || {
            echo "ERROR: expect still not installed after apt." >&2
            exit 1
        }
    fi

    if [[ ! -f "$ISO" ]]; then
        echo "ERROR: $ISO not found. Restore assets/alpine.iso first." >&2
        exit 1
    fi
    [[ -f "$ANSWERS_FILE" ]]  || { echo "ERROR: $ANSWERS_FILE missing"  >&2; exit 1; }
    [[ -f "$POST_INSTALL" ]]  || { echo "ERROR: $POST_INSTALL missing"  >&2; exit 1; }
    [[ -f "$EXPECT_SCRIPT" ]] || { echo "ERROR: $EXPECT_SCRIPT missing" >&2; exit 1; }

    if [[ ! -f "$QCOW2" ]]; then
        echo "[install] creating $QCOW2 ($QCOW2_SIZE, sparse qcow2)..." >&2
        qemu-img create -f qcow2 "$QCOW2" "$QCOW2_SIZE"
    fi

    # The 9p share needs to contain answers + post-install for the expect
    # script to pick up. We mirror them into a stable directory.
    mkdir -p "$SHARE_DIR"
    cp -f "$ANSWERS_FILE" "$SHARE_DIR/alpine-answers.conf"
    cp -f "$POST_INSTALL" "$SHARE_DIR/alpine-rdp-post.sh"
    chmod +x "$SHARE_DIR/alpine-rdp-post.sh"

    cat <<EOF
============================================================
  OblivionOS RDP Preview — INSTALL phase
============================================================
  ISO     : $ISO
  qcow2   : $QCOW2 ($QCOW2_SIZE)
  9p share: $SHARE_DIR   <- read by the in-guest installer
  Expect  : drives Alpine setup-alpine + apk on the serial console

  This phase takes 5–15 minutes end-to-end (mostly apk downloading
  xfce + xrdp inside the guest). You'll see QEMU serial output
  interleaved with script progress as it goes.

  After this finishes, run:
    $0 --boot
============================================================
EOF

    expect "$EXPECT_SCRIPT" "$ISO" "$QCOW2" "$SHARE_DIR"

    cat <<EOF
============================================================
  Install finished.
  qcow2: $QCOW2
  Run:   $0 --boot  to launch the RDP preview.
============================================================
EOF
}

# ---- Boot phase -----------------------------------------------------
QEMU_CLEANED=0
cleanup() {
    local rc=$?
    if [[ $QEMU_CLEANED -eq 1 ]]; then
        return $rc
    fi
    QEMU_CLEANED=1
    echo "" >&2
    echo "[boot] stopping QEMU (pid ${QEMU_PID:-unset})..." >&2
    if [[ -n "${QEMU_PID:-}" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
        kill -TERM "$QEMU_PID" 2>/dev/null || true
        for _ in 1 2 3 4 5 6 7 8 9 10; do
            kill -0 "$QEMU_PID" 2>/dev/null || break
            sleep 0.5
        done
        if kill -0 "$QEMU_PID" 2>/dev/null; then
            echo "[boot] QEMU did not respond to SIGTERM; sending SIGKILL" >&2
            kill -KILL "$QEMU_PID" 2>/dev/null || true
        fi
    fi
    wait "$QEMU_PID" 2>/dev/null || true
    return $rc
}

run_boot_phase() {
    [[ -f "$QCOW2" ]] || {
        echo "ERROR: $QCOW2 not found. Run '$0 --install' first." >&2
        exit 1
    }

    cat <<EOF
============================================================
  OblivionOS Live RDP Preview
============================================================
  qcow2   : $QCOW2
  RDP     : ${RDP_BIND}:${RDP_PORT}     (forward ${RDP_PORT} -> guest:3389)
  Memory  : $MEMORY   Cores: $CORES
  SSH     : host:2222 -> guest:22 (root / no password)
------------------------------------------------------------
  Connect from THIS host:
    Remmina -> File -> New Connection Profile ->
      Protocol: RDP
      Server:   ${RDP_BIND}
      Port:     ${RDP_PORT}
      Username: root
      (leave password empty for the first login)

  Press Ctrl-C in this terminal to stop QEMU cleanly.
============================================================
EOF

    trap cleanup EXIT INT TERM

    qemu-system-x86_64 \
        -m "$MEMORY" \
        -smp "$CORES" \
        -machine accel=kvm:tcg \
        -vga qxl \
        -display none \
        -drive "file=$QCOW2,format=qcow2,if=virtio" \
        -netdev user,id=net0,hostfwd=tcp:${RDP_BIND}:${RDP_PORT}-:3389,hostfwd=tcp:${RDP_BIND}:2222-:22 \
        -device virtio-net-pci,netdev=net0 \
        -name "OblivionOS-RDP" \
        &
    QEMU_PID=$!

    echo "[boot] waiting for RDP listener on ${RDP_BIND}:${RDP_PORT}..." >&2
    listener_up=0
    for _ in $(seq 1 30); do
        if bash -c "exec 3<>/dev/tcp/${RDP_BIND}/${RDP_PORT}" >/dev/null 2>&1; then
            listener_up=1
            break
        fi
        sleep 0.5
    done
    if [[ $listener_up -eq 1 ]]; then
        echo "[boot] RDP listener is up — connect from Remmina now." >&2
    else
        echo "[boot] WARNING: RDP listener did NOT appear within ~15s." >&2
        echo "[boot]          xrdp may still be starting inside the VM; check Remmina in a few seconds." >&2
    fi

    wait "$QEMU_PID"
}

# ---- Dispatch -------------------------------------------------------
if [[ $MODE_INSTALL -eq 1 ]]; then
    run_install_phase
fi
if [[ $MODE_BOOT -eq 1 ]]; then
    run_boot_phase
fi
