#!/bin/sh
# Run the worktree's debug build against fake_service.py under a test bus name.
#   sh target/run_app_fake.sh [fake options...]     e.g. --keyboard-missing, --no-mouse
wt=/var/home/wrichards/Projects/Software/g512-control/.claude/worktrees/keyboard-mouse-views-7d1c
name=io.github.g512control.Lighting1Test
pkill -f "fake_service.py --name $name" 2>/dev/null
pkill -f "$wt/target/flatpak/debug/g512-control" 2>/dev/null
sleep 0.5
python3 -I "$wt/gnome-extension/tests/fake_service.py" --name "$name" "$@" > "$wt/target/fake.log" 2>&1 &
sleep 1
flatpak run --filesystem="$wt" --talk-name="$name" --env=G512_CONTROL_BUS_NAME="$name" \
    --command="$wt/target/flatpak/debug/g512-control" io.github.g512control.Control > "$wt/target/app.log" 2>&1 &
sleep 4
echo "fake: $(head -1 "$wt/target/fake.log")"
