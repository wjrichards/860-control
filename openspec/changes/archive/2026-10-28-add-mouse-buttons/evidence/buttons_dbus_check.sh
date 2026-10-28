#!/bin/sh
# Task 2.3: the button members on a private session bus with no devices
# visible and a throwaway config directory. Run inside the `dev` toolbox
# from the repo root, after `cargo build -p g512d`:
#
#   podman exec -w "$PWD" dev bwrap --dev-bind / / --dev /dev \
#       dbus-run-session -- sh openspec/changes/add-mouse-buttons/evidence/buttons_dbus_check.sh
set -u
cfg=$(mktemp -d)
export XDG_CONFIG_HOME="$cfg"
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"
get() { busctl --user get-property $obj "$@" | tr '\n' ' '; echo; }

./target/debug/g512d > "$cfg/g512d.log" 2>&1 &
pid=$!
for _ in $(seq 50); do
    busctl --user status io.github.g512control.Lighting1 > /dev/null 2>&1 && break
    sleep 0.1
done

echo "hidraw nodes visible: $(ls /dev | grep -c hidraw)"
echo "MouseButtons: $(get MouseButtons)"
echo "SetMouseButton 4, button 8, key 0x06, Ctrl:"
busctl --user call $obj SetMouseButton 'yysqy' 4 8 key 6 1 2>&1 | sed 's/^/  /'
echo "SetMouseButton 4, button 0 (left click):"
busctl --user call $obj SetMouseButton 'yysqy' 4 0 mouse 2 0 2>&1 | sed 's/^/  /'
echo "SetMouseButton 4, button 8, kind other:"
busctl --user call $obj SetMouseButton 'yysqy' 4 8 other 0 0 2>&1 | sed 's/^/  /'
echo "backup file written: $(ls "$cfg/g512-control/mouse-profiles-backup.txt" 2>/dev/null || echo none)"
sleep 2.5
echo "still running after a profile-watch period: $(kill -0 "$pid" 2>/dev/null && echo yes || echo no)"
echo "introspected button members:"
busctl --user introspect io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    | grep -E 'MouseButton' | sed 's/  */ /g; s/^/  /'

kill "$pid"
wait "$pid" 2> /dev/null
rm -rf "$cfg"
