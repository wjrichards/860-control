#!/bin/sh
# Paint 10 times, 20 ms apart, with WASD still selected; capture SetKeys times.
drive=/tmp/claude-1000/-var-home-wrichards-Projects-Software/b0faba46-855d-40b2-82f0-5aa4f5cb7481/scratchpad/a11y_drive.py
out=/tmp/claude-1000/-var-home-wrichards-Projects-Software/b0faba46-855d-40b2-82f0-5aa4f5cb7481/scratchpad/per-key/burst-monitor.txt
timeout -s INT 6 stdbuf -oL busctl --user monitor \
    --match "type='method_call',interface='io.github.g512control.Lighting1',member='SetKeys'" > "$out" 2>&1 &
sleep 0.5
python3 -I "$drive" burst 6 "Paint Selected" 300
sleep 4
grep -o 'Timestamp="[^"]*"' "$out"
