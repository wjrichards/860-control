#!/bin/sh
# Open the editor, switch to the Gaming map and back, listing a few keys.
drive=/tmp/claude-1000/-var-home-wrichards-Projects-Software/b0faba46-855d-40b2-82f0-5aa4f5cb7481/scratchpad/a11y_drive.py
keys='toggle button: (W|Escape|Up|Space),'
python3 -I "$drive" clickrole button "Key Maps"
sleep 0.7
python3 -I "$drive" clickrole "toggle button" "Gaming"
sleep 0.5
echo "--- Gaming:"
python3 -I "$drive" dump | grep -E "$keys" | sed 's/^ *//'
python3 -I "$drive" clickrole "toggle button" "Custom"
sleep 0.5
echo "--- Custom:"
python3 -I "$drive" dump | grep -E "$keys" | sed 's/^ *//'
