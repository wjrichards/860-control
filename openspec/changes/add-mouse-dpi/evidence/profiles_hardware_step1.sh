#!/bin/sh
# Task 2.4, step 1: edit inactive profile 5 through the installed service,
# then compare the backup it saved with spike 6's.
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"
here=$(cd "$(dirname "$0")" && pwd)
backup="$HOME/.config/g512-control/mouse-profiles-backup.txt"
since=$(date '+%H:%M:%S')

echo "backup before: $(ls "$backup" 2>/dev/null || echo none)"
echo "SetMouseProfile 5: 2 ms, default 1, [800 1600 3200]"
busctl --user call $obj SetMouseProfile 'yyyaq' 5 2 1 3 800 1600 3200 2>&1 | sed 's/^/  /'
echo "MouseProfiles: $(busctl --user get-property $obj MouseProfiles)"
echo "MouseActiveProfile: $(busctl --user get-property $obj MouseActiveProfile)"
echo "backup after: $(ls -l "$backup" 2>/dev/null | awk '{print $5, "bytes"}')"
python3 -I - "$backup" "$here/../../archive/2026-10-24-add-mouse-lighting/evidence/spike6-read.txt" <<'EOF'
import re, sys
def sectors(path):
    text = open(path).read()
    return {int(m.group(1), 16): m.group(2) for m in re.finditer(r"sector (0x[0-9a-f]+): [^\n]*\n  ([0-9a-f]+)", text)}
backup, spike6 = sectors(sys.argv[1]), sectors(sys.argv[2])
print("backup sectors:", sorted(backup))
for n in sorted(backup):
    print(f"  sector {n}: equals spike 6 {backup[n] == spike6.get(n)}")
EOF
echo "--- service log:"
journalctl --user -u g512d --since "$since" --no-pager -o cat
