#!/bin/sh
# Task 4.3: record the app's calls to the service while the user works
# through the hands-on steps, then restore the mouse.
#   sh hands_on.sh start     profile 4 active, record method calls for 15 min
#   sh hands_on.sh report    the calls recorded, and the service's view of profile 4
#   sh hands_on.sh restore   write the backup back, make profile 4 active
here=$(cd "$(dirname "$0")" && pwd)
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"
rec="$here/hands_on.monitor"
case "$1" in
start)
    busctl --user call $obj SetMouseActiveProfile y 4 && echo "profile 4 active"
    timeout -s INT 900 stdbuf -oL busctl --user monitor \
        --match "type='method_call',interface='io.github.g512control.Lighting1'" > "$rec" 2>&1 &
    echo "recording from $(date -u +%H:%M:%S) UTC"
    ;;
report)
    echo "--- calls (UTC):"
    awk '/Timestamp=/{t=$0; sub(/.*Timestamp="[^ ]* [^ ]* /, "", t); sub(/ UTC".*/, "", t); sub(/\..*/, "", t)}
         /Member=/{m=$0; sub(/.*Member=/, "", m); sub(/ .*/, "", m)}
         /BYTE|UINT16|STRING/{v=$0; sub(/^ */, "", v); sub(/;$/, "", v); a=a" "v}
         /^$/{if (m) print "  " t, m, a; m=""; a=""}' "$rec" | sed 's/BYTE //g; s/UINT16 //g; s/STRING //g'
    echo "--- profile 4 now:"
    busctl --user get-property $obj MouseButtons | grep -o '4 11 [^5]*' | head -1 | sed 's/^/  /'
    ;;
restore)
    "$HOME/.local/bin/g512d" --restore-mouse-profiles "$HOME/.config/g512-control/mouse-profiles-backup.txt" 2>&1 | sed 's/^/  /'
    busctl --user call $obj SetMouseActiveProfile y 4 && echo "profile 4 active"
    echo "profile 4: $(busctl --user get-property $obj MouseButtons | grep -o '4 11 [^5]*' | head -1)"
    ;;
esac
