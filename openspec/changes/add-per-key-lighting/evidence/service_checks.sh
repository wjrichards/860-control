#!/bin/sh
# Watched service checks for add-per-key-lighting (tasks 1.5, 2.6, 2.7, 3.3).
# Each step prints a timestamp; the keyboard is described in the step label.
set -u
repo=/var/home/wrichards/Projects/Software/g512-control/.claude/worktrees/per-key-lighting-5c2e
corr=$HOME/.config/g512-control/corrections.toml
call() {
    busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
        io.github.g512control.Lighting1 "$@"
}
step() { echo "[$(date +%T.%3N)] $*"; }

G='0 255 0'
WASD="26 $G 4 $G 22 $G 7 $G"
ARROWS="79 $G 80 $G 81 $G 82 $G"

step "S1 SetKeys custom WASD green while Ripple shows: keyboard stays Ripple (4 s)"
call SetKeys 'sa(yyyy)' custom 4 $WASD
sleep 4

step "S2 SetState Custom: WASD green, every other key dark (5 s)"
call SetState 's(yyy)yy' Custom 0 0 0 50 100
sleep 5

step "S3 SetKeys custom WASD + arrows: the arrows turn green too (5 s)"
call SetKeys 'sa(yyyy)' custom 8 $WASD $ARROWS
sleep 5

step "S4 v1-style SetState Static red: whole keyboard red (5 s)"
call SetState 's(yyy)yy' Static 255 0 0 50 100
sleep 5

step "S5 SetKeys custom WASD white while Static shows: keyboard stays red (4 s)"
call SetKeys 'sa(yyyy)' custom 4 26 255 255 255 4 255 255 255 22 255 255 255 7 255 255 255
sleep 4

step "S6 Static white as a zone effect, no corrections: note Left Ctrl (7 s)"
call SetState 's(yyy)yy' Static 255 255 255 50 100
sleep 7

step "S7 corrections.toml e0 = 85, service restarted: Static white as a frame (8 s)"
printf '# Left Ctrl LED is about 15 %% brighter than the rest on this keyboard.\ne0 = 85\n' > "$corr"
systemctl --user restart g512d
sleep 8

step "S8 hardware test: W A S D red, every other key dim white (5 s)"
cargo test -q --manifest-path "$repo/Cargo.toml" -p g512-hidpp --test hardware -- --ignored sets_wasd_frame 2>&1 | grep 'test result'
sleep 5

step "S9 back to Ripple, game switch on, simulated game: orange Gaming map for about 10 s, then Ripple"
call SetState 's(yyy)yy' Ripple 61 56 70 52 100
call SetGameSwitch b true
sleep 2
step "    game starts"
gamemode-simulate-game
step "    game ended"
sleep 4

step "S10 game switch off; Ripple stays"
call SetGameSwitch b false
call SetState 's(yyy)yy' Ripple 61 56 70 52 100
step "done"
