#!/bin/sh
# Task 2.4, step 1: G9 (button 8) on profile 4 becomes Ctrl+C through the
# installed service, then the mouse itself is read back.
here=$(cd "$(dirname "$0")" && pwd)
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"
date -u +"%H:%M:%S UTC"
busctl --user call $obj SetMouseButton 'yysqy' 4 8 key 6 1 && echo "SetMouseButton 4 8 key 6 1: ok"
echo "--- profile 4 as the service offers it:"
busctl --user get-property $obj MouseButtons | grep -o '4 11 [^5]*' | head -1
echo "--- the mouse (profile 5 shown by the script; profile 4 via the journal):"
python3 -I "$here/spike9_media_disabled.py" --status /dev/hidraw3 | head -1
journalctl --user -u g512d -n 2 --no-pager -o cat
