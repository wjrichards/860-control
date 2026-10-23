#!/bin/sh
# Task 1.3: LightsOn and SetLightsOn on a private session bus, with no
# keyboard visible and a throwaway config directory. Run inside the `dev`
# toolbox from the repo root, after `cargo build -p g512d`:
#
#   podman exec -w "$PWD" dev bwrap --dev-bind / / --dev /dev \
#       dbus-run-session -- sh openspec/changes/add-quick-settings-tile/evidence/lights_dbus_check.sh
set -u
cfg=$(mktemp -d)
export XDG_CONFIG_HOME="$cfg"
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"

./target/debug/g512d > "$cfg/g512d.log" 2>&1 &
pid=$!
for _ in $(seq 50); do
    busctl --user status io.github.g512control.Lighting1 > /dev/null 2>&1 && break
    sleep 0.1
done

echo "hidraw nodes visible: $(ls /dev | grep -c hidraw)"
echo "DeviceState: $(busctl --user get-property $obj DeviceState)"
echo "LightsOn at start: $(busctl --user get-property $obj LightsOn)"

timeout 3 busctl --user monitor --match "type='signal',member='PropertiesChanged'" > "$cfg/signals.txt" 2>&1 &
sleep 0.5
busctl --user call $obj SetLightsOn b false
sleep 0.5
echo "LightsOn after SetLightsOn false: $(busctl --user get-property $obj LightsOn)"
echo "signal carried LightsOn=false: $(grep -A2 '"LightsOn"' "$cfg/signals.txt" | grep -c 'BOOLEAN false')"
echo "saved: $(grep lights_on "$cfg/g512-control/state.toml")"

busctl --user call $obj SetState 's(yyy)yy' Static 0 0 255 50 100
echo "LightsOn after SetState: $(busctl --user get-property $obj LightsOn)"
echo "saved: $(grep lights_on "$cfg/g512-control/state.toml")"

kill "$pid"
wait "$pid" 2> /dev/null
echo "--- service log:"
cat "$cfg/g512d.log"
rm -rf "$cfg"
