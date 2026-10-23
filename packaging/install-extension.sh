#!/bin/sh
# Install the Quick Settings tile (a GNOME Shell 50 extension) for the
# current user. GNOME Shell on Wayland loads a newly installed extension at
# the next login; after logging out and in, enable it:
#
#   gnome-extensions enable g512-control@g512control.github.io
#
#   packaging/install-extension.sh             pack and install
#   packaging/install-extension.sh --uninstall remove it (the Shell must be running)
set -eu

uuid=g512-control@g512control.github.io
repo=$(cd "$(dirname "$0")/.." && pwd)

if [ "${1:-}" = "--uninstall" ]; then
    gnome-extensions uninstall "$uuid"
    echo "Removed $uuid."
    exit 0
fi

out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
gnome-extensions pack --force --out-dir "$out" --extra-source="$repo/gnome-extension/logic.js" "$repo/gnome-extension"
gnome-extensions install --force "$out/$uuid.shell-extension.zip"
echo "Installed $uuid to ${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/$uuid."

if [ "$(gsettings get org.gnome.shell disable-user-extensions 2> /dev/null)" = "true" ]; then
    echo
    echo "Extensions are turned off for this account. Turn them on in the Extensions app, or with:"
    echo "  gsettings set org.gnome.shell disable-user-extensions false"
fi
echo
echo "Log out and back in, then: gnome-extensions enable $uuid"
