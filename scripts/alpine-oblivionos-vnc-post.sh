#!/bin/sh
# scripts/alpine-oblivionos-vnc-post.sh
#
# Runs INSIDE the Alpine guest after setup-alpine finishes, as the very last
# step of the expect script. Installs:
#   * xfce4 (light WM with a real taskbar + decoration — gives us a window
#     to see VNC frames move against)
#   * x11vnc (the in-guest VNC server that bridges X11 -> VNC for clients
#     that connect to QEMU's -vnc :0 exposition. We export :0 from x11vnc
#     and bind it on the same port QEMU listens on.)
#   * a tiny systemd-style openrc init script that launches x11vnc on boot
#     in front of xfce
#   * the eventual hooks for oblivion-compositor (commented out for now,
#     since the compositor doesn't run on Alpine/musl glibc binaries yet)
#
# Idempotency: safe to re-run. If xfce is already installed, apk returns 0
# in non-strict mode without re-downloading.

set -e

echo "[post] apk update..."
apk update --no-cache 2>&1 | sed 's/^/  /'

echo "[post] apk add xfce4 x11vnc ..."
apk add --no-cache \
    xfce4 \
    xfce4-terminal \
    x11vnc \
    openbox \
    dbus \
    dbus-x11 \
    sudo \
    bash 2>&1 | sed 's/^/  /'

# localuser: group 'wheel' keeps sudo working; create a no-password user.
echo "[post] ensure local user 'oblivion' can sudo without password..."
if ! id oblivion >/dev/null 2>&1; then
    adduser -D -s /bin/bash -G wheel oblivion || true
fi
# sed -i without -e so it works on busybox sed (Alpine)
sed -i 's/^# %wheel ALL=(ALL) NOPASSWD: ALL/%wheel ALL=(ALL) NOPASSWD: ALL/' \
    /etc/sudoers 2>/dev/null || true

# x11vnc service script. OpenRC convention: /etc/init.d/<name>, enabled via
# rc-update add. x11vnc shows :0 (whatever X display XFCE eventually
# creates) on 127.0.0.1:5900 of the guest. QEMU's -vnc :0 doesn't expose
# the in-guest X yet, so we also listen locally and rely on QEMU's user-mode
# net forwarding for the 'preview' path that doesn't use -vnc. The make
# preview-oblivionos-vnc target uses -vnc :0 directly, so the in-guest
# x11vnc is a fallback for the same qcow2 if the host forgets -vnc.
echo "[post] install x11vnc init script..."
install -m755 /dev/stdin /etc/init.d/x11vnc <<'XEOF'
#!/sbin/openrc-run

description="x11vnc bridge exposing X session as VNC :0 on guest"
command="/usr/bin/x11vnc"
command_args="-display :0 -rfbport 5900 -localhost -forever -shared -quiet"
command_background="yes"
pidfile="/run/x11vnc.pid"
depend() {
    need localmount
    use xdm
}

start_pre() {
    # ensure X0 sockets' group is readable by x11vnc
    mkdir -p /tmp/.X11-unix 2>/dev/null || true
    chmod 1777 /tmp/.X11-unix || true
}
XEOF

rc-update add x11vnc default 2>&1 | sed 's/^/  /'

# xdm autologin to XFCE
echo "[post] configure xdm to start XFCE on boot..."
if [[ ! -f /etc/init.d/xdm ]]; then
    apk add --no-cache xdm 2>&1 | sed 's/^/  /'
fi
rc-update add xdm default 2>&1 | sed 's/^/  /'
# Alpine's xdm defaults to twm; switch to XFCE
if [[ -f /etc/X11/xdm/Xsession ]]; then
    echo 'XSESSION=/usr/bin/startxfce4' > /etc/X11/xdm/Xsession.env 2>/dev/null || true
fi

# Make the boot screen visibility obvious (so the test/VNC preview
# distinguishes this qcow2 from the xrdp one).
cat > /etc/motd <<'MOTDEOF'
  ____  _      ___ _____ ___  ___
 / _ \ | | /| / _ \_   _/ _ \/ __|
| | | || |/ || | | || || | | (__
| |_| ||   / | |_| || || |_| |\__ \
 \___/ |_|_\_|\___/ |_| \___/|___/

  OblivionOS cloud-image (VNC preview build)
  - in-guest VNC on 127.0.0.1:5900 (x11vnc)
  - XDM auto-starts XFCE4 on boot
MOTDEOF

# Optional hook for when oblivion-compositor becomes runnable on musl.
# We pre-create a systemd-style init that will silently no-op while the
# binary isn't present; that way the in-guest layout doesn't need to
# change when Stage 3 of the VNC roadmap lands.
install -m755 /dev/stdin /etc/init.d/oblivion-compositor 2>/dev/null <<'HOOKEOF' || true
#!/sbin/openrc-run
description="OblivionOS Rust compositor (scaffold; disabled if binary missing)"
command="/usr/local/bin/oblivion-compositor"
command_background="yes"
pidfile="/run/oblivion-compositor.pid"
depend() { need xdm; }
HOOKEOF

echo "[post] done."
