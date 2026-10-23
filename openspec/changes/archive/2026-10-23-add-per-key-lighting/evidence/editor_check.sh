#!/bin/sh
# Open the editor, select WASD, paint, then burst-paint; capture SetKeys calls.
drive=/tmp/claude-1000/-var-home-wrichards-Projects-Software/b0faba46-855d-40b2-82f0-5aa4f5cb7481/scratchpad/a11y_drive.py
out=/tmp/claude-1000/-var-home-wrichards-Projects-Software/b0faba46-855d-40b2-82f0-5aa4f5cb7481/scratchpad/per-key/editor-monitor.txt
timeout -s INT 12 stdbuf -oL busctl --user monitor \
    --match "type='method_call',interface='io.github.g512control.Lighting1',member='SetKeys'" > "$out" 2>&1 &
sleep 0.5
python3 -I "$drive" clickrole button "Key Maps"
sleep 0.7
echo "--- editor page:"
python3 -I "$drive" dump | grep -E 'toggle button|push button|button:' | sed 's/^ *//' | head -14
echo "toggle buttons: $(python3 -I "$drive" dump | grep -c 'toggle button:')"
python3 -I "$drive" clickrole button "WASD"
sleep 0.3
echo "--- selected after WASD:"
python3 -I "$drive" dump | grep '\[checked\]' | sed 's/^ *//'
python3 -I "$drive" clickrole button "Paint Selected"
sleep 1.5
echo "--- after paint:"
python3 -I "$drive" dump | grep -E 'toggle button: (W|A|S|D|Q),' | sed 's/^ *//'
python3 -I "$drive" burst 10 "Paint Selected"
sleep 9
