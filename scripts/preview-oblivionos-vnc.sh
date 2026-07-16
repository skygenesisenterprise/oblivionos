#!/bin/bash
# scripts/preview-oblivionos-vnc.sh — Boot a persistent OblivionOS VNC cloud-image.
#
# What this gives you, end-to-end:
#   1. one-time `--install` builds assets/oblivionos-vnc.qcow2 with Alpine v3.20
#      installed headless via serial-console expect + setup-alpine, then
#      layer-loaded with: xfce4 (a real desktop window manager), x11vnc (the
#      in-guest VNC bridge that surfaces the X session as :0 / port 5900 on
#      127.0.0.1 of the guest), openrc autostart for both, and a placeholder
#      init script for the Rust oblivion-compositor.
#   2. every-time `--boot` boots that qcow2 with QEMU's built-in VNC
#      server on ${VNC_BIND}:${VNC_PORT}. Connect from THIS host with
#      Remmina / TigerVNC / Vinagre / wayvnc.
#
# Default ports (override with env):
#   VNC_BIND    127.0.0.1   # dashed-quad IPv4 only (QEMU requirement)
#   VNC_PORT    5900        # standard VNC display :0; default for OBLIVION
#   MEMORY      2G
#   CORES       2
#   QCOW2_SIZE  6G          # sparse, only the install phase actually grows it
#
# This mirrors scripts/preview-rdp.sh in spirit (two-phase install + boot,
# same Alpine + xfce base, same cleanup pattern) but switches the
# reachable-from-host layer:
#   preview-rdp     -> hostfwd 3389 -> xrdp           (Remmina RDP)
#   preview-osp-vnc -> QEMU -vnc  :0  -> in-guest X    (Remmina/TigerVNC/...)
#
# Why two phases? 'setup-alpine' needs an interactive serial console for
# ten or so prompts; instead of writing a brittle Python state machine
# we keep the simple expect pattern from the RDP variant and run it once,
# after which the qcow2 is reusable for every preview session.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ASSETS_DIR="$PROJECT_DIR/assets"

ISO="$ASSETS_DIR/alpine.iso"
QCOW2="$ASSETS_DIR/oblivionos-vnc.qcow2"
SHARE_DIR="$ASSETS_DIR/share-vnc"
ANSWERS_FILE="$SCRIPT_DIR/alpine-oblivionos-vnc-answers.conf"
POST_INSTALL="$SCRIPT_DIR/alpine-oblivionos-vnc-post.sh"
EXPECT_SCRIPT="$SCRIPT_DIR/alpine-oblivionos-vnc-install.exp"

MEMORY="${MEMORY:-2G}"
CORES="${CORES:-2}"
VNC_BIND="${VNC_BIND:-127.0.0.1}"
VNC_PORT="${VNC_PORT:-5900}"
VNC_DISPLAY_N="${VNC_DISPLAY_N:-$((VNC_PORT - 5900))}"
QCOW2_SIZE="${QCOW2_SIZE:-6G}"

# ---- VNC_BIND validation: dotted-quad IPv4 only. Reject hostnames/empty
# early so QEMU doesn't error late with 'Could not set up host forwarding rule'.
if ! [[ "$VNC_BIND" =~ ^[0-9]+(\.[0-9]+){3}$ ]]; then
    cat >&2 <<EOF
ERROR: VNC_BIND='$VNC_BIND' is not a literal IPv4 dotted-quad address.

  Examples of valid values: 127.0.0.1, 0.0.0.0, 192.168.1.10
  Examples of invalid values: localhost, ::1, 10.0.0

  Set VNC_BIND via env var or pass it on the command line.
EOF
    exit 1
fi

mkdir -p "$ASSETS_DIR"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo "ERROR: qemu-system-x86_64 not found. Run: sudo apt install qemu-system-x86" >&2
    exit 1
fi

# ---- Mode selection ------------------------------------------------
mode="${1:-auto}"
case "$mode" in
    auto)
        if [[ ! -f "$QCOW2" ]]; then
            MODE_INSTALL=1; MODE_BOOT=0
        else
            MODE_INSTALL=0; MODE_BOOT=1
        fi
        ;;
    --install) MODE_INSTALL=1; MODE_BOOT=0 ;;
    --boot)    MODE_INSTALL=0; MODE_BOOT=1 ;;
    *)
        cat >&2 <<EOF
Usage: $0 {auto|--install|--boot}

  auto      install if qcow2 missing, otherwise boot (default)
  --install one-time install via expect + setup-alpine + apk (xfce + x11vnc)
  --boot    launch the qcow2 with QEMU's built-in VNC on ${VNC_BIND}:${VNC_PORT}

Env overrides: VNC_BIND, VNC_PORT, MEMORY, CORES, QCOW2_SIZE.
EOF
        exit 1
        ;;
esac

# ---- Install phase ------------------------------------------------
run_install_phase() {
    if ! command -v expect >/dev/null 2>&1; then
        echo "[install] expect not found; installing via apt..." >&2
        if command -v sudo >/dev/null 2>&1; then
            sudo apt install -y expect >&2 || { echo "ERROR: 'sudo apt install expect' failed." >&2; exit 1; }
        else
            apt install -y expect >&2 || { echo "ERROR: 'apt install expect' failed and sudo is missing." >&2; exit 1; }
        fi
    fi

    [[ -f "$ISO" ]]           || { echo "ERROR: $ISO not found. Restore assets/alpine.iso first." >&2; exit 1; }
    [[ -f "$ANSWERS_FILE" ]]  || { echo "ERROR: $ANSWERS_FILE missing"  >&2; exit 1; }
    [[ -f "$POST_INSTALL" ]]  || { echo "ERROR: $POST_INSTALL missing"  >&2; exit 1; }
    [[ -f "$EXPECT_SCRIPT" ]] || { echo "ERROR: $EXPECT_SCRIPT missing" >&2; exit 1; }

    if [[ ! -f "$QCOW2" ]]; then
        echo "[install] creating $QCOW2 ($QCOW2_SIZE, sparse)..." >&2
        qemu-img create -f qcow2 "$QCOW2" "$QCOW2_SIZE"
    fi

    mkdir -p "$SHARE_DIR"
    cp -f "$ANSWERS_FILE"  "$SHARE_DIR/alpine-oblivionos-vnc-answers.conf"
    cp -f "$POST_INSTALL"  "$SHARE_DIR/alpine-oblivionos-vnc-post.sh"
    chmod +x "$SHARE_DIR/alpine-oblivionos-vnc-post.sh"

    cat <<EOF
============================================================
  OblivionOS VNC Preview — INSTALL phase
============================================================
  ISO         : $ISO
  qcow2       : $QCOW2 ($QCOW2_SIZE)
  9p share    : $SHARE_DIR
  Post-install: $POST_INSTALL
============================================================
  Expect will drive Alpine setup-alpine via the serial console and then
  run alpine-oblivionos-vnc-post.sh (Xfce + x11vnc + openrc).

  After this finishes, run:
    $0 --boot      (or: make preview-oblivionos-vnc)
============================================================
EOF

    expect "$EXPECT_SCRIPT" "$ISO" "$QCOW2" "$SHARE_DIR"

    cat <<EOF
============================================================
  Install finished.
  qcow2: $QCOW2
  Run:   $0 --boot
============================================================
EOF
}

# ---- Boot phase ---------------------------------------------------
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
            echo "[boot] QEMU did not respond to SIGTERM; SIGKILL" >&2
            kill -KILL "$QEMU_PID" 2>/dev/null || true
        fi
    fi
    wait "$QEMU_PID" 2>/dev/null || true
    return $rc
}

run_boot_phase() {
    [[ -f "$QCOW2" ]] || { echo "ERROR: $QCOW2 not found. Run '$0 --install' first." >&2; exit 1; }

    cat <<EOF
============================================================
  OblivionOS Live VNC Preview
============================================================
  qcow2   : $QCOW2
  VNC     : ${VNC_BIND}:${VNC_PORT}   (display :${VNC_DISPLAY_N})
  Memory  : $MEMORY   Cores: $CORES
  SSH     : host:2222 -> guest:22 (root)
------------------------------------------------------------
  Connect from THIS host with any of:
    Remmina       : File -> New Connection Profile,
                    Protocol=VNC, Server=${VNC_BIND}, Port=${VNC_PORT}
    Quick Connect : vnc://${VNC_BIND}:${VNC_PORT}
    TigerVNC      : vncviewer ${VNC_BIND}:${VNC_PORT}
    Vinagre       : (open the URL above)
    wayvnc-client : wayvnc-client ${VNC_BIND}:${VNC_PORT}

  Press Ctrl-C in this terminal to stop QEMU cleanly.
============================================================
EOF

    trap cleanup EXIT INT TERM

    qemu-system-x86_64 \
        -m "$MEMORY" \
        -smp "$CORES" \
        -machine accel=kvm:tcg \
        -vga qxl \
        -display vnc="${VNC_BIND}:${VNC_DISPLAY_N}" \
        -drive file="$QCOW2",format=qcow2,if=virtio \
        -netdev user,id=net0,hostfwd=tcp::2222-:22 \
        -device virtio-net-pci,netdev=net0 \
        -name "OblivionOS-VNC" \
        &
    QEMU_PID=$!

    echo "[boot] waiting for VNC listener on ${VNC_BIND}:${VNC_PORT}..." >&2
    listener_up=0
    for _ in $(seq 1 30); do
        if bash -c "exec 3<>/dev/tcp/${VNC_BIND}/${VNC_PORT}" >/dev/null 2>&1; then
            listener_up=1
            break
        fi
        sleep 0.5
    done
    if [[ $listener_up -eq 1 ]]; then
        echo "[boot] VNC listener is up — connect from your client now." >&2
    else
        echo "[boot] WARNING: VNC listener did NOT appear within ~15s." >&2
        echo "[boot]          xfce/x11vnc may still be starting inside the VM; try your client in a few seconds." >&2
    fi

    wait "$QEMU_PID"
}

# ---- Dispatch -----------------------------------------------------
if [[ $MODE_INSTALL -eq 1 ]]; then run_install_phase; fi
if [[ $MODE_BOOT    -eq 1 ]]; then run_boot_phase;    fi
