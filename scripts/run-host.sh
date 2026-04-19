#!/bin/bash
# OblivionOS - Test local (sur votre machine)

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "=== OblivionOS Test Local ==="

# Build
echo "Building..."
cd "$PROJECT_DIR"
cargo build --release

# Detect display
if [[ -n "$DISPLAY" ]]; then
    export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"
fi

if [[ -n "$WAYLAND_DISPLAY" ]]; then
    echo "Using Wayland: $WAYLAND_DISPLAY"
    export DISPLAY=""
else
    echo "Using X11: ${DISPLAY:-:0}"
    export DISPLAY="${DISPLAY:-:0}"
fi

echo "Starting compositor..."
./target/release/oblivion-compositor &
COMP_PID=$!
sleep 1

echo "Starting shell..."
./target/release/oblivion-shell &
SHELL_PID=$!

echo ""
echo "=== Started ==="
echo "Compositor PID: $COMP_PID"
echo "Shell PID: $SHELL_PID"
echo ""
echo "Si une fenetre s'ouvre, le test succes!"
echo ""
echo "Pour arreter:"
echo "  kill $COMP_PID $SHELL_PID"

# Wait for user or Ctrl+C
trap "kill $COMP_PID $SHELL_PID 2>/dev/null; exit" INT TERM

wait