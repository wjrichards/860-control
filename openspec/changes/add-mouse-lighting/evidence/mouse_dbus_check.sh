#!/bin/sh
# Task 2.4: the mouse members on a private session bus, with no devices
# visible and a throwaway config directory. Run inside the `dev` toolbox from
# the repo root, after `cargo build -p g512d`:
#
#   podman exec -w "$PWD" dev bwrap --dev-bind / / --dev /dev \
#       dbus-run-session -- sh openspec/changes/add-mouse-lighting/evidence/mouse_dbus_check.sh
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
echo "start: MouseState $(get MouseState)MouseName $(get MouseName)MouseEffects $(get MouseEffects)"
echo "       MouseMatch $(get MouseMatch)MouseLighting $(get MouseLighting)"

busctl --user call $obj SetState 's(yyy)yy' Ripple 61 56 70 52 100 > /dev/null
echo "keyboard set to Ripple: MouseLighting (default own) $(get MouseLighting)"

timeout 3 busctl --user monitor --match "type='signal',member='PropertiesChanged'" > "$cfg/signals.txt" 2>&1 &
sleep 0.5
echo "SetMouseLighting Static blue: pending $(busctl --user call $obj SetMouseLighting 's(yyy)yy' Static 0 0 255 50 80)"
sleep 0.5
echo "  MouseMatch $(get MouseMatch)MouseLighting $(get MouseLighting)LightsOn $(get LightsOn)"
echo "  signal carried MouseMatch=false: $(grep -A2 '"MouseMatch"' "$cfg/signals.txt" | grep -c 'BOOLEAN false')"
echo "  saved [mouse]:"
sed -n '/^\[mouse\]/,/^\[/p' "$cfg/g512-control/state.toml" | sed 's/^/    /'

busctl --user call $obj SetMouseMatch b true
echo "SetMouseMatch true: MouseMatch $(get MouseMatch)saved $(grep match_keyboard "$cfg/g512-control/state.toml")"

echo "SetMouseLighting Ripple (not a mouse effect):"
busctl --user call $obj SetMouseLighting 's(yyy)yy' Ripple 0 0 255 50 80 2>&1 | sed 's/^/  /'

kill "$pid"
wait "$pid" 2> /dev/null
echo "--- service log:"
cat "$cfg/g512d.log"
rm -rf "$cfg"
