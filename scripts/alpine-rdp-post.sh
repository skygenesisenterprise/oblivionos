#!/bin/sh
# scripts/alpine-rdp-post.sh — runs inside the freshly-installed Alpine
# qcow2 to install xfce + xrdp. Executed by the expect installer over the
# 9p share; not run directly by a human.
set -e

# Mount the host's 9p share (mounted automatically by setup if present).
if ! mountpoint -q /mnt/share 2>/dev/null; then
    mount -t 9p -o trans=virtio,version=9p2000.L hostshare /mnt/share \
        || true
fi

# Re-point apk at a working mirror in case the answers-file default is slow.
setup-apkrepos -c -1 -k 2>/dev/null || true

# Required packages for an xfce4 desktop reachable over RDP.
apk add \
    xfce4 \
    xfce4-session \
    xfce4-terminal \
    thunar \
    xorg \
    xrdp \
    terminus-font \
    ttf-dejavu \
    dbus \
    polkit \
    adwaita-icon-theme \
    papirus-icon-theme

# Configure X session via xrdp's startwm.sh hook.
cat > /etc/xrdp/startwm.sh <<'WM'
#!/bin/sh
if test -r /etc/profile; then
    . /etc/profile
fi
exec startxfce4
WM
chmod +x /etc/xrdp/startwm.sh

# Allow root inside the RDP sesman (useful for first-login testing).
# Both BusyBox sed (Alpine default) and GNU sed accept the POSIX character
# class `[[:space:]]` for whitespace; `\s` is GNU-only and would silently
# no-op on BusyBox. The grep below appends the key if sed failed because
# the line was wholly absent (some xrdp builds ship without it).
sed -i 's/^#\?[[:space:]]*AllowRootLogin=.*/AllowRootLogin=true/' /etc/xrdp/sesman.ini
grep -q '^AllowRootLogin=' /etc/xrdp/sesman.ini \
    || printf '%s\n' 'AllowRootLogin=true' >> /etc/xrdp/sesman.ini

# OpenRC services.
rc-update add dbus default
rc-update add xrdp default
rc-update add xrdp-sesman default

# Make sure the install will boot with graphics (no systemd here).
rc-update add udev sysinit
rc-update add cgroups sysinit

# Done. Reboot to bring everything up cleanly.
sync
reboot
