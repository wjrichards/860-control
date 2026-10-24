#!/bin/sh
# Task 2.4, steps 4 and 5: switch profiles through the service, then
# restore the backup with g512d --restore-mouse-profiles and check that the
# mouse and the running service both match it again.
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"
here=$(cd "$(dirname "$0")" && pwd)
mouse=$1
backup="$HOME/.config/g512-control/mouse-profiles-backup.txt"
since=$(date '+%H:%M:%S')
read_dpi() { python3 -I "$here/read_dpi.py" "$mouse"; }

echo "start: $(read_dpi)"
busctl --user call $obj SetMouseActiveProfile y 1
echo "SetMouseActiveProfile 1: MouseActiveProfile $(busctl --user get-property $obj MouseActiveProfile), mouse: $(read_dpi)"
busctl --user call $obj SetMouseActiveProfile y 4
echo "SetMouseActiveProfile 4: MouseActiveProfile $(busctl --user get-property $obj MouseActiveProfile), mouse: $(read_dpi)"

echo "--- restore:"
"$HOME/.local/bin/g512d" --restore-mouse-profiles "$backup" 2>&1 | sed 's/^/  /'
echo "after restore: MouseProfiles $(busctl --user get-property $obj MouseProfiles)"
echo "mouse: $(read_dpi)"
python3 -I - "$backup" "$here/../../archive/2026-10-24-add-mouse-lighting/evidence/spike6-read.txt" <<'EOF'
import re, sys
def sectors(path):
    text = open(path).read()
    return {int(m.group(1), 16): m.group(2) for m in re.finditer(r"sector (0x[0-9a-f]+): [^\n]*\n  ([0-9a-f]+)", text)}
backup, spike6 = sectors(sys.argv[1]), sectors(sys.argv[2])
print("backup still equals spike 6:", all(backup[n] == spike6[n] for n in backup))
EOF
echo "--- service log:"
journalctl --user -u g512d --since "$since" --no-pager -o cat
