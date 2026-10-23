#!/bin/sh
# Run the tile in a nested GNOME Shell against tests/fake_service.py, then
# source a check script inside that session. Run in the `dev` toolbox:
#
#   toolbox run -c dev sh gnome-extension/tests/nested.sh CHECK_SCRIPT OUT_DIR [--devkit]
#
# Nothing here touches the real session: the Shell gets its own session bus
# (dbus-run-session), its own XDG_RUNTIME_DIR, in-memory GSettings and
# temporary XDG data, config, cache and state directories. Headless with a
# virtual monitor by default; --devkit shows it in a window instead.
#
# The check script can use: shell_eval JS, screenshot NAME, fake_set NAME
# GVARIANT, calls (the fake service's call log), and $out.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
uuid=g512-control@g512control.github.io
helper=unsafe-helper@g512control.github.io

if [ "${1:-}" != --inside ]; then
    check=$(realpath "$1")
    out=$(realpath -m "$2")
    mode=${3:-}
    mkdir -p "$out"
    tmp=$(mktemp -d)
    mkdir -m 700 "$tmp/run"
    ext="$tmp/data/gnome-shell/extensions"
    mkdir -p "$ext/$uuid" "$ext/$helper"
    cp "$here/../metadata.json" "$here/../extension.js" "$here/../logic.js" "$ext/$uuid/"
    cp "$here/unsafe-helper/metadata.json" "$here/unsafe-helper/extension.js" "$ext/$helper/"
    status=0
    env -i HOME="$HOME" USER="$USER" PATH=/usr/bin:/bin LANG=C.UTF-8 \
        XDG_RUNTIME_DIR="$tmp/run" XDG_DATA_HOME="$tmp/data" XDG_CONFIG_HOME="$tmp/config" \
        XDG_CACHE_HOME="$tmp/cache" XDG_STATE_HOME="$tmp/state" GSETTINGS_BACKEND=memory \
        HOST_WAYLAND="${XDG_RUNTIME_DIR:-}/${WAYLAND_DISPLAY:-}" \
        dbus-run-session -- sh "$0" --inside "$check" "$out" "$mode" || status=$?
    # The session's document portal may still have its FUSE mount here.
    fusermount3 -uz "$tmp/run/doc" 2> /dev/null || true
    rm -rf "$tmp" 2> /dev/null || { sleep 1; rm -rf "$tmp"; }
    exit "$status"
fi

check=$2
out=$3
mode=$4

shell_eval() {
    gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
        --method org.gnome.Shell.Eval "$1"
}
screenshot() {
    gdbus call --session --dest org.gnome.Shell.Screenshot --object-path /org/gnome/Shell/Screenshot \
        --method org.gnome.Shell.Screenshot.Screenshot false false "$out/$1.png" > /dev/null
}
fake_set() {
    gdbus call --session --dest io.github.g512control.Lighting1 --object-path /io/github/g512control/Lighting1 \
        --method io.github.g512control.Test.Set "$1" "$2" > /dev/null
}
# The fake service's log of calls from the tile: "<seconds> call <Method> <args>".
calls() {
    grep ' call ' "$out/service.log" || true
}

python3 -I "$here/fake_service.py" --yes-this-is-a-private-bus > "$out/service.log" 2>&1 &
fake=$!
if [ "$mode" = --devkit ]; then
    WAYLAND_DISPLAY=$HOST_WAYLAND gnome-shell --devkit --wayland --no-x11 > "$out/shell.log" 2>&1 &
else
    gnome-shell --headless --virtual-monitor 1280x800 --wayland --no-x11 --wayland-display=g512-test \
        > "$out/shell.log" 2>&1 &
fi
shell=$!
cleanup() {
    kill "$shell" 2> /dev/null || true
    kill "$fake" 2> /dev/null || true
    wait 2> /dev/null || true
}
trap cleanup EXIT

i=0
until gnome-extensions list > /dev/null 2>&1; do
    i=$((i + 1))
    if [ "$i" -gt 300 ] || ! kill -0 "$shell" 2> /dev/null; then
        echo "the nested Shell did not start; see $out/shell.log" >&2
        exit 1
    fi
    sleep 0.1
done
gnome-extensions enable "$helper"
gnome-extensions enable "$uuid"
sleep 1
# Out of the way of screenshots: the first-login tour, the overview, and
# the "unsafe mode" notification.
shell_eval "Main.layoutManager.modalDialogGroup.get_children().forEach(d => d.close?.()); \
    Main.overview.hide(); Main.messageTray.getSources().forEach(s => s.destroy()); 'tidy'" > /dev/null
sleep 1

. "$check"
