#!/bin/sh
# Install g512d for the current user: binary in ~/.local/bin, a systemd user
# unit, and a D-Bus activation file. No root needed; the udev rule is
# separate (see packaging/INSTALL-udev.md).
#
#   packaging/install-service.sh            build, install, enable and start
#   packaging/install-service.sh --uninstall stop, disable and remove
set -eu

repo=$(cd "$(dirname "$0")/.." && pwd)
bindir="$HOME/.local/bin"
unitdir="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
dbusdir="${XDG_DATA_HOME:-$HOME/.local/share}/dbus-1/services"
dbusfile="$dbusdir/io.github.g512control.Lighting1.service"

if [ "${1:-}" = "--uninstall" ]; then
    systemctl --user disable --now g512d.service 2>/dev/null || true
    rm -f "$unitdir/g512d.service" "$dbusfile" "$bindir/g512d"
    systemctl --user daemon-reload
    echo "g512d removed. Saved lighting is kept in ${XDG_CONFIG_HOME:-$HOME/.config}/g512-control/."
    exit 0
fi

cargo build --release --manifest-path "$repo/Cargo.toml" -p g512d

install -Dm0755 "$repo/target/release/g512d" "$bindir/g512d"
install -Dm0644 "$repo/packaging/systemd/g512d.service" "$unitdir/g512d.service"
mkdir -p "$dbusdir"
sed "s|@BINDIR@|$bindir|" "$repo/packaging/dbus/io.github.g512control.Lighting1.service.in" > "$dbusfile"

systemctl --user daemon-reload
systemctl --user enable g512d.service
systemctl --user restart g512d.service

if [ ! -e /etc/udev/rules.d/70-g512-control.rules ]; then
    echo
    echo "The udev rule is not installed, so g512d cannot open the keyboard yet:"
    echo "  sudo install -m 0644 $repo/packaging/70-g512-control.rules /etc/udev/rules.d/"
    echo "  sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=hidraw"
fi
systemctl --user --no-pager status g512d.service | head -n 5
