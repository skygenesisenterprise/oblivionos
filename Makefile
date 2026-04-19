# OblivionOS Makefile

.PHONY: help all clean build dev

help:
	@echo "OblivionOS Build System"
	@echo ""
	@echo "Usage:"
	@echo "  make all          - Build all packages"
	@echo "  make release      - Build release version"
	@echo "  make dev          - Run in dev mode (needs Wayland)"
	@echo ""
	@echo "  QEMU Options (set DISPLAY_TYPE):"
	@echo "    make run-qemu                    # GTK display"
	@echo "    DISPLAY_TYPE=vnc make run-qemu   # VNC display"
	@echo "    DISPLAY_TYPE=spice make run-qemu # SPICE display"
	@echo ""
	@echo "  QEMU parameters:"
	@echo "    MEMORY=4G CORES=4 make run-qemu  # Custom memory/cores"

all:
	cargo build

release:
	cargo build --release

dev:
	@echo "Running compositor..."
	WAYLAND_DISPLAY=wayland-0 cargo run --package oblivion-compositor

run-qemu:
	@echo "Launching QEMU..."
	@bash scripts/run-qemu.sh

clean:
	cargo clean

test:
	cargo test --all