#!/bin/sh
# Task 2.3: the profile members on a private session bus with no devices
# visible and a throwaway config directory. Run inside the `dev` toolbox
# from the repo root, after `cargo build -p g512d`:
#
#   podman exec -w "$PWD" dev bwrap --dev-bind / / --dev /dev \
#       dbus-run-session -- sh openspec/changes/add-mouse-dpi/evidence/profiles_dbus_check.sh
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
echo "MouseProfiles: $(get MouseProfiles)"
echo "MouseActiveProfile: $(get MouseActiveProfile)"
echo "MouseDpiRange: $(get MouseDpiRange)"
echo "SetMouseProfile 4, 1 ms, default 0, [1600 3200]:"
busctl --user call $obj SetMouseProfile 'yyyaq' 4 1 0 2 1600 3200 2>&1 | sed 's/^/  /'
echo "SetMouseActiveProfile 1:"
busctl --user call $obj SetMouseActiveProfile y 1 2>&1 | sed 's/^/  /'
echo "backup file written: $(ls "$cfg/g512-control/mouse-profiles-backup.txt" 2>/dev/null || echo none)"
printf 'sector 0x0001: crc ok\n  00\n' > "$cfg/fake-backup.txt"
echo "g512d --restore-mouse-profiles with no mouse:"
./target/debug/g512d --restore-mouse-profiles "$cfg/fake-backup.txt" > "$cfg/restore.txt" 2>&1
status=$?
sed 's/^/  /' "$cfg/restore.txt"
echo "  exit status $status"

kill "$pid"
wait "$pid" 2> /dev/null
rm -rf "$cfg"
