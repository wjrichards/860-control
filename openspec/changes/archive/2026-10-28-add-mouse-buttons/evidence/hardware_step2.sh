#!/bin/sh
# Task 2.4, step 2: make profile 5 active (its G9 cycles profiles), and
# record the service's property signals for 90 s while the user presses G9.
#   sh hardware_step2.sh start    switch and start recording
#   sh hardware_step2.sh report   show the journal and the recorded signals
here=$(cd "$(dirname "$0")" && pwd)
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"
rec="$here/hardware_step2.monitor"
case "$1" in
start)
    timeout -s INT 90 stdbuf -oL busctl --user monitor \
        --match "type='signal',interface='org.freedesktop.DBus.Properties',path='/io/github/g512control/Lighting1'" \
        > "$rec" 2>&1 &
    sleep 0.5
    busctl --user call $obj SetMouseActiveProfile y 5 && echo "SetMouseActiveProfile 5: ok at $(date -u +%H:%M:%S) UTC"
    echo "service: $(busctl --user get-property $obj MouseActiveProfile)"
    ;;
report)
    echo "--- journal:"
    journalctl --user -u g512d --since "-5min" --no-pager -o short-precise | grep -E "switched|profile" | sed 's/^/  /'
    echo "--- PropertiesChanged signals (UTC) carrying MouseActiveProfile:"
    awk '/Timestamp=/{t=$0; sub(/.*Timestamp="[^ ]* [^ ]* /, "", t); sub(/ UTC".*/, "", t)}
         /STRING "MouseActiveProfile"/{getline; getline; v=$0; gsub(/[^0-9]/, "", v); print "  " t, "MouseActiveProfile =", v}' "$rec"
    echo "service now: $(busctl --user get-property $obj MouseActiveProfile)"
    echo "mouse now: $(python3 -I "$here/spike9_media_disabled.py" --status /dev/hidraw3 | head -1)"
    ;;
esac
