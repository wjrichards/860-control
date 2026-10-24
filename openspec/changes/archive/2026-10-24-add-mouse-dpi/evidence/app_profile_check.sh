#!/bin/sh
# Task 3.1: drive the app's Mouse Profile group over AT-SPI and time the
# write. Activates "Add Step" once, records the calls, then restores the
# backup so profile 4 is as before.
here=$(cd "$(dirname "$0")" && pwd)
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"
mon=$(mktemp)

echo "before: $(busctl --user get-property $obj MouseProfiles)"
timeout -s INT 8 stdbuf -oL busctl --user monitor \
    --match "type='method_call',interface='io.github.g512control.Lighting1'" > "$mon" 2>&1 &
sleep 0.5
start=$(date +%s.%N)
python3 -I "$here/app_profile_a11y.py" act "Add Step" 2>&1 | grep -v -i deprecat
echo "Add Step activated at $(date +%H:%M:%S.%N | cut -c1-12)"
sleep 6
echo "--- calls (seconds after Add Step):"
awk -v s="$start" '/Timestamp=/{t=$0; sub(/.*Timestamp="[^ ]* [^ ]* /, "", t); sub(/ UTC".*/, "", t)}
     /Member=/{m=$0; sub(/.*Member=/, "", m); sub(/ .*/, "", m)}
     /BYTE|UINT16|ARRAY/{v=v" "$0}
     /^$/{if (m) print "  " t, m, v; m=""; v=""}' "$mon" | sed 's/  */ /g'
echo "after: $(busctl --user get-property $obj MouseProfiles)"
rm -f "$mon"
echo "--- restore:"
"$HOME/.local/bin/g512d" --restore-mouse-profiles "$HOME/.config/g512-control/mouse-profiles-backup.txt" 2>&1 | sed 's/^/  /'
echo "restored: $(busctl --user get-property $obj MouseProfiles)"
