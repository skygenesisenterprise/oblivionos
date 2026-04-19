#!/bin/bash
# Simple QEMU launcher pour OblivionOS
# Lance le compositor et shell directement

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="$PROJECT_DIR/target/release"

echo "=== OblivionOS Launcher ==="

# Verifier que les binaires existent
if [[ ! -f "$BUILD_DIR/oblivion-compositor" ]]; then
    echo "Building..."
    cargo build --release
fi

# Configuration
MEMORY="${MEMORY:-4G}"
CORES="${CORES:-4}"

echo "Memory: $MEMORY"
echo "Cores: $CORES"
echo ""

# Methode 1: Via dispaclay GTK simple
echo "Option 1: Display GTK (plus simple)"
echo "  qemu-system-x86_64 -m $MEMORY -smp $CORES -kernel /boot/vmlinuz -initrd /boot/initrd.img"
echo ""

# Methode 2: Via Wayland socket
echo "Option 2: Avec Wayland dans VM"
echo "  1. Start qemu avec wayland support"
echo "  2. Dans la VM: export WAYLAND_DISPLAY=wayland-0"
echo "  3. ./oblivion-compositor &"
echo "  4. ./oblivion-shell &"
echo ""

# Methode 3: Via network
echo "Option 3: Via network (pour dev)"
echo "  qemu-system-x86_64 -m $MEMORY -smp $CORES"
echo "  -netdev user,id=net0,hostfwd=tcp::2222-:22"
echo "  -device e1000,netdev=net0"
echo ""

read -p "Choisir method (1/2/3) ou Enter pour demarrer QEMU: " choice

case "$choice" in
    1)
        echo "Launching with GTK display..."
        qemu-system-x86_64 \
            -m "$MEMORY" \
            -smp "$CORES" \
            -display gtk \
            -enable-kvm 2>/dev/null || \
        qemu-system-x86_64 \
            -m "$MEMORY" \
            -smp "$CORES" \
            -display gtk
        ;;
    2)
        echo "Lancer manuellement dans VM:"
        echo "  WAYLAND_DISPLAY=wayland-0 ./target/release/oblivion-compositor"
        echo "  WAYLAND_DISPLAY=wayland-0 ./target/release/oblivion-shell"
        ;;
    3)
        echo "Lancer avec network..."
        qemu-system-x86_64 \
            -m "$MEMORY" \
            -smp "$CORES" \
            -netdev user,id=net0,hostfwd=tcp::2222-:22 \
            -device e1000,netdev=net0 \
            -display gtk
        ;;
    *)
        echo "Usage standard:"
        echo "  Pour testing rapide:"
        echo "    qemu-system-x86_64 -m 4G -smp 4 -display gtk"
        echo ""
        echo "  Pour testing avec acceleration:"
        echo "    qemu-system-x86_64 -m 4G -smp 4 -enable-kvm -display gtk"
        ;;
esac