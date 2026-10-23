#!/bin/sh
# For each effect, set it from outside and list which rows the app shows.
drive=/tmp/claude-1000/-var-home-wrichards-Projects-Software/b0faba46-855d-40b2-82f0-5aa4f5cb7481/scratchpad/a11y_drive.py
for effect in Off Static Breathe Cycle Wave Ripple Custom; do
    busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
        io.github.g512control.Lighting1 SetState 's(yyy)yy' "$effect" 61 56 70 52 100 > /dev/null
    sleep 0.8
    rows=$(python3 -I "$drive" dump | grep 'list item:' | sed 's/.*list item: //' | grep -v '^G512' | tr '\n' ',' | sed 's/,$//')
    echo "$effect: $rows"
done
busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
    io.github.g512control.Lighting1 SetState 's(yyy)yy' Ripple 61 56 70 52 100 > /dev/null
