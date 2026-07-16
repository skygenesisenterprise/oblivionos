#!/bin/bash
# OblivionOS QEMU - Boot the installer ISO with a configurable remote display.
#
# Display mode is controlled by the env var DISPLAY_TYPE (default: gtk):
#
#   DISPLAY_TYPE=gtk                 local GTK window (default; what you'd use
#                                    when sitting at the host machine)
#   DISPLAY_TYPE=vnc                 QEMU's built-in VNC server. Connect with
#                                    Remmina / TigerVNC / Vinagre / wayvnc.
#                                    Honours VNC_BIND, VNC_PORT, VNC_PASSWORD.
#   DISPLAY_TYPE=spice               QEMU's SPICE server. Connect with Remmina
#                                    (SPICE plugin) / remote-viewer / virt-viewer.
#                                    Honours SPICE_BIND, SPICE_PORT, SPICE_PASSWORD.
#   DISPLAY_TYPE=none                headless; useful in CI for unattended
#                                    installs via serial + ssh -p 2222.
#
# Forward tickets in addition to the display:
#   MEMORY=4G CORES=4 bash scripts/run-qemu.sh          # 4 GiB / 4 vCPU
#   DISPLAY_TYPE=vnc bash scripts/run-qemu.sh          # VNC, display :0
#   VNC_BIND=0.0.0.0 VNC_PORT=5901 bash scripts/run-qemu.sh
#   DISPLAY_TYPE=spice SPICE_BIND=0.0.0.0 bash scripts/run-qemu.sh
#
# This is the v0 of the new "preview as a Debian ISO installer" story:
# boot the Ubuntu ISO from assets/ and reach its installer with any VNC client.

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ASSETS_DIR="$PROJECT_DIR/assets"

echo "=== OblivionOS QEMU ==="

mkdir -p "$ASSETS_DIR"
DISK="$ASSETS_DIR/oblivionos.qcow2"
ISO="$ASSETS_DIR/ubuntu.iso"

# Ubuntu URL
UBUNTU_URL="https://releases.ubuntu.com/24.04/ubuntu-24.04-live-server-amd64.iso"

# ---- Boot parameters (env-overridable) ---------------------------
MEMORY="${MEMORY:-4G}"
CORES="${CORES:-4}"
DISPLAY_TYPE="${DISPLAY_TYPE:-gtk}"
VNC_BIND="${VNC_BIND:-127.0.0.1}"
VNC_PORT="${VNC_PORT:-5900}"
# VNC_DISPLAY_N is the "display number" suffix on QEMU's -vnc arg (default 0).
# QEMU appends 5900+display. We derive VNC_PORT from this unless set explicitly.
if [[ -n "${VNC_DISPLAY_N:-}" && -z "${VNC_PORT_OVERRIDE:-}" ]]; then
  VNC_PORT=$((5900 + VNC_DISPLAY_N))
fi
VNC_PASSWORD="${VNC_PASSWORD:-}"
SPICE_BIND="${SPICE_BIND:-127.0.0.1}"
SPICE_PORT="${SPICE_PORT:-5900}"
SPICE_PASSWORD="${SPICE_PASSWORD:-}"

# ---- Disk ----------------------------------------------------------
if [[ ! -f "$DISK" ]]; then
    echo "Creating disk (20GB)..."
    qemu-img create -f qcow2 "$DISK" 20G
fi

# ---- ISO -----------------------------------------------------------
# A real Ubuntu 24.04 server ISO is ~2 GB; the desktop one ~5 GB.
# Anything <500 MB is either an HTML error page (CDN 404 behind a
# HTTP-200 wrapper, an HTTP redirect to a soft 404, or a misconfigured
# mirror response) or a release-notes file. `curl --fail` makes a
# non-2xx status abort (set -euo pipefail) so we never silently save
# what was just a 282-byte HTML page.
MIN_ISO_BYTES=500000000

if [[ -f "$ISO" ]] && [[ $(stat -c%s "$ISO") -lt "$MIN_ISO_BYTES" ]]; then
    echo "Existing $ISO is smaller than 500 MB (likely an error page); removing." >&2
    rm -f "$ISO"
fi

if [[ ! -f "$ISO" ]]; then
    echo "Downloading Ubuntu Server from $UBUNTU_URL..."
    TMP_ISO="$(mktemp -t obliv-iso.XXXXXX)"
    if ! curl --fail --location --connect-timeout 30 --max-time 900 \
              --output "$TMP_ISO" "$UBUNTU_URL"; then
        cat >&2 <<EOF
ERROR: failed to download from
  $UBUNTU_URL

  Likely causes:
    - outbound HTTPS blocked (corporate proxy / firewall)
    - the URL has changed (try testing with: curl -I '$UBUNTU_URL')
    - the mirror is down

  Workarounds:
    1. override the URL:
         UBUNTU_URL=https://path/to/your.iso bash scripts/run-qemu.sh
    2. pre-place a real ISO at $ISO (>=500 MB) and re-run
EOF
        rm -f "$TMP_ISO"
        exit 1
    fi
    if [[ $(stat -c%s "$TMP_ISO") -lt "$MIN_ISO_BYTES" ]]; then
        echo "ERROR: $TMP_ISO is only $(stat -c%s "$TMP_ISO") bytes — curl returned an HTML error page despite HTTP 2xx." >&2
        echo "       Try a different UBUNTU_URL or pre-place the ISO." >&2
        rm -f "$TMP_ISO"
        exit 1
    fi
    mv "$TMP_ISO" "$ISO"
fi

QEMU_BIN=""
# ---- QEMU binary discovery -------------------------------------
# On Ubuntu hosts where the snap-package core20 libs clash with the
# host glibc, `/usr/bin/qemu-system-x86_64` (apt) can fail with:
#   symbol lookup error: …/libpthread.so.0: undefined symbol:
#   __libc_pthread_init, version GLIBC_PRIVATE
# Prefer the snap-bundled self-contained qemu (if installed) so we
# sidestep the loader conflict entirely. Probe via `ldd`: any line
# reading 'not found' means the binary will die on dlopen.
for cand in /snap/bin/qemu-system-x86_64 /usr/bin/qemu-system-x86_64 /bin/qemu-system-x86_64; do
    [[ -x "$cand" ]] || continue
    if ldd "$cand" 2>&1 | grep -q 'not found'; then
        echo "[diag] $cand: ldd reports unresolved symbols; skipping" >&2
        continue
    fi
    QEMU_BIN="$cand"
    break
done
if [[ -z "$QEMU_BIN" ]]; then
    QEMU_BIN="$(command -v qemu-system-x86_64 2>/dev/null || true)"
fi
if [[ -z "$QEMU_BIN" || ! -x "$QEMU_BIN" ]]; then
    cat >&2 <<'EOF'
ERROR: no working qemu-system-x86_64 binary found.

  Your system /usr/bin/qemu-system-x86_64 (or the snap one) crashes on
  launch with:
    symbol lookup error: …/libpthread.so.0: undefined symbol:
    __libc_pthread_init, version GLIBC_PRIVATE

  That's a snap core20 vs apt glibc version mismatch. Two fixes:
    1. install the snap-bundled self-contained qemu (recommended):
         sudo snap install qemu
    2. force-reinstall the apt qemu so it links against libpthread
       from the same glibc it was compiled against:
         sudo apt remove --purge qemu-system-x86
         sudo apt install qemu-system-x86
EOF
    exit 1
fi
echo "[diag] QEMU binary : $QEMU_BIN"
# Sanity probe: a broken GLIBC_PRIVATE binary prints usage then dies
# inside dlopen; running -version is cheap and lets us fail with a
# clean error BEFORE we'd otherwise create the disk and download the
# ISO.
if ! "$QEMU_BIN" -version >/dev/null 2>&1; then
    echo "ERROR: $QEMU_BIN -version failed; binary cannot start (likely snap-vs-apt glibc)." >&2
    echo "       Try: sudo snap install qemu" >&2
    exit 1
fi

# ---- Build QEMU options based on DISPLAY_TYPE ---------------------
case "$DISPLAY_TYPE" in
    gtk)
        DISPLAY_ARGS="-display gtk"
        BANNER_DISPLAY="GTK (local window)"
        BANNER_CLIENT_HINT="no remote client required"
        ;;
    vnc)
        # QEMU's VNC flag combinations:
        #   `-display vnc=[host]:display`        — open, no password knob
        #   `-vnc [host]:display[,password]`     — accepts ,password token
        #                                       — requires `-passwordfile`
        #                                         with 8-byte DES-encrypted
        #                                         payload (annoying to hand-
        #                                         craft from a POSIX shell).
        #   `-vnc [host]:display,password=P`     — DEPRECATED but works on
        #                                         QEMU 2.12+: inline plain-
        #                                         text password. We use
        #                                         this for ergonomic env-
        #                                         var support; QEMU logs a
        #                                         deprecation warning.
        if ! [[ "$VNC_BIND" =~ ^[0-9]+(\.[0-9]+){3}$ ]]; then
            echo "ERROR: VNC_BIND='$VNC_BIND' is not a literal IPv4 dotted-quad." >&2
            echo "       Examples: 127.0.0.1, 0.0.0.0, 192.168.1.10" >&2
            exit 1
        fi
        VNC_DISPLAY_N="${VNC_DISPLAY_N:-$((VNC_PORT - 5900))}"
        VNC_TOKEN="${VNC_BIND}:${VNC_DISPLAY_N}"
        if [[ -n "$VNC_PASSWORD" ]]; then
            # QEMU parses `-vnc <token-list>` by splitting on ',' and '=',
            # so a user-set VNC_PASSWORD containing those characters would
            # be reinterpreted as additional option tokens by QEMU
            # (e.g. ',to=42' switching to a different display number).
            # Reject rather than silently mangling.
            if [[ "$VNC_PASSWORD" == *[,=]* ]]; then
                echo "ERROR: VNC_PASSWORD may not contain ',' or '=' (QEMU token separators)." >&2
                echo "       Choose a password using [A-Za-z0-9._-] only." >&2
                exit 1
            fi
            VNC_TOKEN+=",password=${VNC_PASSWORD}"
        fi
        DISPLAY_ARGS="-display none -vnc ${VNC_TOKEN}"
        BANNER_DISPLAY="VNC on ${VNC_BIND}:${VNC_PORT}"
        BANNER_CLIENT_HINT="Remmina / TigerVNC / Vinagre / wayvnc"
        ;;
    spice)
        if [[ -n "$SPICE_PASSWORD" ]]; then
            SPICE_ARGS="port=${SPICE_PORT},addr=${SPICE_BIND},password=${SPICE_PASSWORD}"
        else
            SPICE_ARGS="port=${SPICE_PORT},addr=${SPICE_BIND},disable-ticketing=on"
        fi
        DISPLAY_ARGS="-spice ${SPICE_ARGS} -display none"
        BANNER_DISPLAY="SPICE on ${SPICE_BIND}:${SPICE_PORT}"
        BANNER_CLIENT_HINT="Remmina (SPICE plugin) / remote-viewer / virt-viewer"
        ;;
    none)
        DISPLAY_ARGS="-display none -nographic"
        BANNER_DISPLAY="headless (serial only)"
        BANNER_CLIENT_HINT="ssh -p 2222 root@127.0.0.1"
        ;;
    *)
        echo "ERROR: unknown DISPLAY_TYPE='$DISPLAY_TYPE'." >&2
        echo "       Valid: gtk | vnc | spice | none" >&2
        exit 1
        ;;
esac

# ---- Banner -------------------------------------------------------
iso_size=$(stat -c%s "$ISO" 2>/dev/null || echo "?")
echo "ISO     : $ISO ($iso_size bytes)"
echo "Disk    : $DISK"
echo "Memory  : $MEMORY   Cores: $CORES"
echo "Display : $BANNER_DISPLAY"
echo "Connect : $BANNER_CLIENT_HINT"

case "$DISPLAY_TYPE" in
    vnc)
        cat <<EOF

  --- VNC connection examples from THIS host ---
    Remmina       : File -> New Connection Profile,
                    Protocol=VNC, Server=${VNC_BIND}, Port=${VNC_PORT}
    Quick Connect : vnc://${VNC_BIND}:${VNC_PORT}
    TigerVNC      : vncviewer ${VNC_BIND}:${VNC_PORT}
    Vinagre       : ssh -X ${USER}@localhost vinagre vnc://${VNC_BIND}:${VNC_PORT}
    wayvnc-client : wayvnc-client ${VNC_BIND}:${VNC_PORT}
EOF
        if [[ -n "$VNC_PASSWORD" ]]; then
            echo "  Password     : $VNC_PASSWORD (inline PLAINTEXT; QEMU logs a deprecation notice)"
        fi
        ;;
    spice)
        cat <<EOF

  --- SPICE connection examples from THIS host ---
    Remmina       : File -> New Connection Profile,
                    Protocol=SPICE, Server=${SPICE_BIND}, Port=${SPICE_PORT}
    Quick Connect : spice://${SPICE_BIND}:${SPICE_PORT}
    remote-viewer : spice://${SPICE_BIND}:${SPICE_PORT}
EOF
        if [[ -n "$SPICE_PASSWORD" ]]; then
            echo "  Password     : $SPICE_PASSWORD"
        fi
        ;;
esac

echo

# ---- Launch -------------------------------------------------------
if [[ -f "$ISO" ]]; then
    if [[ "$DISPLAY_TYPE" == "gtk" ]]; then
        echo "Starting QEMU with a local GTK window..."
    fi
    "$QEMU_BIN" \
        -m "$MEMORY" \
        -smp "$CORES" \
        $DISPLAY_ARGS \
        -vga std \
        -drive file="$DISK",format=qcow2 \
        -cdrom "$ISO" \
        -boot d \
        -net nic \
        -net user,hostfwd=tcp::2222-:22 \
        -name "OblivionOS" \
        "$@"
else
    echo "No ISO - download from:"
    echo "https://ubuntu.com/download/server"
fi

echo "Done"
