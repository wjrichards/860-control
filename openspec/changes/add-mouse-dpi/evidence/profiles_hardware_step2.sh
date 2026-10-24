#!/bin/sh
# Task 2.4, step 2: edit the active profile 4 (250 Hz, steps 800, 1600,
# 3200, starting on 800). The user then tries the DPI buttons.
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"
since=$(date '+%H:%M:%S')
echo "MouseState $(busctl --user get-property $obj MouseState), active $(busctl --user get-property $obj MouseActiveProfile)"
echo "SetMouseProfile 4: 4 ms, default 0, [800 1600 3200]"
busctl --user call $obj SetMouseProfile 'yyyaq' 4 4 0 3 800 1600 3200 2>&1 | sed 's/^/  /'
echo "MouseProfiles: $(busctl --user get-property $obj MouseProfiles)"
echo "--- service log:"
journalctl --user -u g512d --since "$since" --no-pager -o cat
