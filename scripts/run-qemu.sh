#!/bin/bash
# OblivionOS QEMU - Boot from Ubuntu

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ASSETS_DIR="$PROJECT_DIR/assets"

echo "=== OblivionOS QEMU ==="

mkdir -p "$ASSETS_DIR"
DISK="$ASSETS_DIR/oblivionos.qcow2"
ISO="$ASSETS_DIR/ubuntu.iso"

# Ubuntu URL
UBUNTU_URL="https://releases.ubuntu.com/24.04/ubuntu-24.04-live-server-amd64.iso"

# Create disk
if [[ ! -f "$DISK" ]]; then
    echo "Creating disk (20GB)..."
    qemu-img create -f qcow2 "$DISK" 20G
fi

# Check/clean ISO
if [[ -f "$ISO" ]] && [[ $(stat -c%s "$ISO") -lt 1000000000 ]]; then
    rm -f "$ISO"
fi

if [[ ! -f "$ISO" ]]; then
    echo "Downloading Ubuntu Server..."
    curl -L -o "$ISO" "$UBUNTU_URL" --max-time 600
fi

QEMU_BIN="/bin/qemu-system-x86_64"

if [[ -f "$ISO" ]]; then
    echo "ISO size: $(stat -c%s "$ISO") bytes"
    echo "Starting QEMU..."
    echo "Connect avec VNC: localhost:5900"
    echo ""
    "$QEMU_BIN" \
        -m 4G \
        -smp 4 \
        -display vnc=:0 \
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