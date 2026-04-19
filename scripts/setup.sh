#!/bin/bash
# OblivionOS - Installation et test

set -e

echo "=== OblivionOS Setup ==="

# Methode 1: Installer QEMU
install_qemu() {
    echo "Installing QEMU..."
    sudo apt update
    sudo apt install -y qemu-system-x86 qemu-utils
}

# Methode 2: Tester sans VM (localhost display)
run_local() {
    echo "Testing locally (requires Wayland)..."
    
    # Verifier Wayland
    if [[ -z "$WAYLAND_DISPLAY" ]]; then
        if [[ -S "/run/user/$(id -u)/wayland-0" ]]; then
            export WAYLAND_DISPLAY=wayland-0
        else
            echo "ERROR: No Wayland display found"
            echo "Pour tester, utilisez un display server Wayland"
            exit 1
        fi
    fi
    
    echo "WAYLAND_DISPLAY=$WAYLAND_DISPLAY"
    
    # Build
    cargo build
    
    # Run
    echo "Starting compositor..."
    ./target/debug/oblivion-compositor &
    echo "Starting shell..."
    ./target/debug/oblivion-shell &
    
    echo "Services started"
}

# Methode 3: Creer disk image
create_disk() {
    local size="${1:-20G}"
    echo "Creating disk image ($size)..."
    mkdir -p assets
    qemu-img create -f qcow2 assets/oblivionos.qcow2 "$size"
    echo "Created: assets/oblivionos.qcow2"
}

# Menu
case "${1:-}" in
    install)
        install_qemu
        ;;
    local|run)
        run_local
        ;;
    disk)
        create_disk "${2:-20G}"
        ;;
    *)
        cat << 'EOF'
Usage: ./scripts/setup.sh [COMMAND]

Commands:
    install   - Install QEMU
    local     - Run locally (needs Wayland)
    disk [SIZE] - Create disk image

Pour QEMU complet:
    1. Run: sudo apt install qemu-system-x86 qemu-utils
    2. Run: make run-qemu

EOF
        ;;
esac