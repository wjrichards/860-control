#!/bin/sh
# Run the worktree's debug build against fake_service.py under a test bus name.
#   sh target/run_app_fake.sh [fake options...]     e.g. --keyboard-missing, --no-mouse
# SHOT, VIEW and SIZE set the screenshot hooks; FAKE_CALL is a ready gdbus prefix.
wt=/var/home/wrichards/Projects/Software/g512-control/.claude/worktrees/keyboard-mouse-views-7d1c
name=io.github.g512control.Lighting1Test
ps -eo pid,args | awk -v n="$name" '/[f]ake_service.py --name/ && index($0, n) {print $1}' | xargs -r kill
ps -eo pid,args | awk '/[d]ebug\/g512-control/ {print $1}' | xargs -r kill
sleep 0.5
python3 -I "$wt/gnome-extension/tests/fake_service.py" --name "$name" "$@" > "$wt/target/fake.log" 2>&1 &
sleep 1
# PRE: a shell snippet run against the fake before the app starts, e.g.
#   PRE='gdbus call ... SetState Static "(38, 162, 105)" 50 100'
[ -n "$PRE" ] && sh -c "$PRE" > /dev/null
flatpak run --filesystem="$wt" --talk-name="$name" --env=G512_CONTROL_BUS_NAME="$name" \
    ${SHOT:+--env=G512_CONTROL_SCREENSHOT=$SHOT} ${VIEW:+--env=G512_CONTROL_VIEW=$VIEW} ${SIZE:+--env=G512_CONTROL_SIZE=$SIZE} \
    --command="$wt/target/flatpak/debug/g512-control" io.github.g512control.Control > "$wt/target/app.log" 2>&1 &
sleep 4
echo "fake: $(head -1 "$wt/target/fake.log")"
