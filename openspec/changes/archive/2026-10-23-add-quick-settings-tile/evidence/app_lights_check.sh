#!/bin/sh
# Task 2.1: the app's Lights switch over AT-SPI, against the installed
# service. Starts with the lights off and leaves them on.
#   sh app_lights_check.sh /path/to/a11y_drive.py
drive=$1
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"
mon=$(mktemp)
row() { python3 -I "$drive" dump | grep -E '^ *switch: Lights' | head -n 1 | sed 's/^ *//'; }
lights() { busctl --user get-property $obj LightsOn; }

echo "start: service LightsOn $(lights), app row '$(row)'"

timeout -s INT 4 stdbuf -oL busctl --user monitor \
    --match "type='method_call',interface='io.github.g512control.Lighting1'" > "$mon" 2>&1 &
sleep 0.5
python3 -I "$drive" clickrole switch Lights
sleep 3.5
echo "flipped the row on: calls $(grep -o 'Member=[A-Za-z]*' "$mon" | tr '\n' ' ')args $(grep -o 'BOOLEAN [a-z]*' "$mon" | tr '\n' ' ')"
echo "  service LightsOn $(lights), app row '$(row)'"

busctl --user call $obj SetLightsOn b false
sleep 1
echo "busctl SetLightsOn false: app row '$(row)'"
busctl --user call $obj SetLightsOn b true
sleep 1
echo "busctl SetLightsOn true: app row '$(row)'"
rm -f "$mon"
