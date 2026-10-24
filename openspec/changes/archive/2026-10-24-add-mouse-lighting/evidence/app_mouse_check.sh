#!/bin/sh
# Task 3.1: the app's Mouse section over AT-SPI, against the installed
# service with the mouse connected. Starts with Match Keyboard off and ends
# with it on. The service's properties and the recorded calls are the
# evidence: AT-SPI's checked state lags behind the widget.
here=$(cd "$(dirname "$0")" && pwd)
drive="python3 -I $here/app_mouse_a11y.py"
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"
mon=$(mktemp)
prop() { busctl --user get-property $obj "$1"; }
own_rows() { $drive rows | grep -E '^(combo box: Effect|list item: (Colour|Speed|Brightness))' | sed 's/ \[.*//' | tr '\n' ',' ; echo; }

timeout -s INT 40 stdbuf -oL busctl --user monitor \
    --match "type='method_call',interface='io.github.g512control.Lighting1'" > "$mon" 2>&1 &
sleep 0.5

echo "1. start: MouseMatch $(prop MouseMatch); own-setting rows: $(own_rows)"

$drive set "Mouse Brightness" 60
sleep 1.5
echo "2. Mouse Brightness set to 60 in the app: MouseLighting $(prop MouseLighting)"

$drive act "Match Keyboard" switch
sleep 1.5
echo "3. Match Keyboard toggled: MouseMatch $(prop MouseMatch); rows: $(own_rows)"

$drive act "Match Keyboard" switch
sleep 1.5
echo "4. toggled again: MouseMatch $(prop MouseMatch); rows: $(own_rows)"

$drive set "Mouse Brightness" 100
sleep 1.5
echo "5. Mouse Brightness back to 100: MouseLighting $(prop MouseLighting)"

$drive act "Match Keyboard" switch
sleep 1.5
echo "6. toggled again: MouseMatch $(prop MouseMatch); rows: $(own_rows)"

sleep 1
echo "--- calls to the service, in order:"
awk '/Member=/{m=$0; sub(/.*Member=/, "", m); sub(/ .*/, "", m)} /STRING "|BOOLEAN|BYTE/{v=v" "$0} /^$/{if (m) print "  " m v; m=""; v=""}' "$mon" \
    | sed 's/  */ /g'
rm -f "$mon"
