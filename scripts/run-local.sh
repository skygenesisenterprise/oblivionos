#!/bin/bash
# Launch OblivionOS directly on host (sans VM)
# Pour testing du compositor sans QEMU

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_DIR/target/release"

echo "=== OblivionOS Direct Launch ==="

# Build si necessaire
if [[ ! -f "$BUILD_DIR/oblivion-compositor" ]]; then
    echo "Building..."
    cd "$PROJECT_DIR"
    cargo build --release
fi

# Detecter Wayland display
if [[ -z "$WAYLAND_DISPLAY" ]]; then
    if [[ -S "$XDG_RUNTIME_DIR/wayland-0" ]]; then
        export WAYLAND_DISPLAY=wayland-0
    else
        export WAYLAND_DISPLAY=wayland-0
    fi
fi

echo "WAYLAND_DISPLAY=$WAYLAND_DISPLAY"

# Launch compositor
echo "Starting compositor..."
"$BUILD_DIR/oblivion-compositor" &
COMPITOR_PID=$!

sleep 1

# Launch shell
echo "Starting shell..."
"$BUILD_DIR/oblivion-shell" &
SHELL_PID=$!

echo ""
echo "Compositor PID: $COMPITOR_PID"
echo "Shell PID: $SHELL_PID"
echo ""
echo "Pour arreter:"
echo "  kill $COMPITOR_PID $SHELL_PID"

# Wait
wait