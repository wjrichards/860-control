#!/bin/sh
# Task 4.3: record every call to the lighting service for SECONDS (default
# 600) into OUT, so calls from the tile (sender: gnome-shell) can be told
# apart from the app's (sender: the Flatpak's xdg-dbus-proxy).
#   sh record_calls.sh OUT [SECONDS]
exec timeout -s INT "${2:-600}" stdbuf -oL busctl --user monitor \
    --match "type='method_call',interface='io.github.g512control.Lighting1'" > "$1" 2>&1
