# OblivionOS Makefile

.PHONY: help all clean build dev preview preview-vnc preview-oblivionos-vnc preview-oblivionos-vnc-install preview-rdp preview-rdp-install run-qemu test

help:
	@echo "OblivionOS Build System"
	@echo ""
	@echo "Usage:"
	@echo "  make all             - Build all packages"
	@echo "  make release         - Build release version"
	@echo "  make dev             - Run in dev mode (needs Wayland)"
	@echo ""
	@echo "  Local QEMU preview (no remote display):"
	@echo "  make run-qemu                  # GTK window"
	@echo ""
	@echo "  Remote-display QEMU previews (Debian-style installer preview):"
	@echo "  make preview                  # Alpine via SPICE (remmina/virt-viewer)"
	@echo "  make preview-vnc              # Ubuntu ISO via QEMU built-in VNC"
	@echo "                                  (Remmina / TigerVNC / Vinagre / wayvnc)"
	@echo "  make preview-rdp-install      # one-time: build persistent qcow2 with"
	@echo "                                  Alpine + xfce + xrdp (5-15 min)"
	@echo "  make preview-rdp              # boot that qcow2 with xrdp forward on 3389"
	@echo "  make preview-oblivionos-vnc-install   # one-time: persistent qcow2 with"
	@echo "                                          Alpine + xfce + x11vnc (5-15 min)"
	@echo "  make preview-oblivionos-vnc   # boot that qcow2, expose -vnc :0"
	@echo ""
	@echo "  DISPLAY_TYPE for run-qemu (default: gtk):"
	@echo "    DISPLAY_TYPE=vnc   make run-qemu      # QEMU built-in VNC"
	@echo "    DISPLAY_TYPE=spice make run-qemu      # QEMU SPICE"
	@echo "    DISPLAY_TYPE=none  make run-qemu      # headless (serial + ssh)"
	@echo ""
	@echo "  QEMU/RDP/VNC/SPICE env knobs (override on the same line):"
	@echo "    MEMORY=4G CORES=4   make <target>      # RAM / vCPU"
	@echo "    VNC_BIND=0.0.0.0    make preview-vnc   # expose VNC on LAN"
	@echo "    VNC_PORT=5901       make preview-vnc   # custom VNC port"
	@echo "    VNC_PASSWORD=secret make preview-vnc   # require a VNC password"
	@echo "    SPICE_BIND=0.0.0.0  make preview       # expose SPICE on LAN"
	@echo "    RDP_BIND=0.0.0.0    make preview-rdp   # expose RDP on LAN"
	@echo "    RDP_PORT=3389       make preview-rdp   # canonical RDP port"

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

preview:
	@echo "Launching SPICE preview (legacy)..."
	@bash scripts/preview.sh

preview-vnc:
	@echo "Launching VNC preview (Remmina / TigerVNC / Vinagre / wayvnc)..."
	@DISPLAY_TYPE=vnc bash scripts/run-qemu.sh

preview-oblivionos-vnc-install:
	@echo "Installing Alpine + xfce + x11vnc into assets/oblivionos-vnc.qcow2 ..."
	@bash scripts/preview-oblivionos-vnc.sh --install

preview-oblivionos-vnc:
	@echo "Booting the OblivionOS VNC cloud-image..."
	@bash scripts/preview-oblivionos-vnc.sh --boot

preview-rdp-install:
	@echo "Installing Alpine + xfce + xrdp into assets/oblivionos-rdp.qcow2 ..."
	@bash scripts/preview-rdp.sh --install

preview-rdp:
	@echo "Launching RDP preview on 127.0.0.1:3389 ..."
	@bash scripts/preview-rdp.sh --boot

clean:
	cargo clean

test:
	cargo test --all
