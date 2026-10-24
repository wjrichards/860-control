#!/bin/sh
# Task 2.6: mouse lighting on the real G502 HERO with the installed g512d,
# while the user watches. About 80 s. Restores the user's state at the end.
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"
call() { busctl --user call $obj "$@" > /dev/null; }
get() { busctl --user get-property $obj "$@" | tr '\n' ' '; }
since=$(date '+%H:%M:%S')
t0=$(date +%s)
step() { echo "[$(( $(date +%s) - t0 ))s] $*"; }
saved=$(busctl --user get-property $obj State | sed 's/^(s(yyy)yy) //')

step "start: MouseState $(get MouseState)MouseMatch $(get MouseMatch)State $saved"
call SetMouseMatch b true

step "1  keyboard Static red -> mouse RED"
call SetState 's(yyy)yy' Static 255 0 0 50 100; sleep 4
step "2  keyboard Breathe green -> mouse BREATHES GREEN"
call SetState 's(yyy)yy' Breathe 0 255 0 50 100; sleep 7
step "3  keyboard Cycle slow (speed 10) -> mouse CYCLES SLOWLY, same pace as the keyboard"
call SetState 's(yyy)yy' Cycle 0 0 0 10 100; sleep 7
step "4  keyboard Cycle fast (speed 100) -> mouse CYCLES FAST, same pace"
call SetState 's(yyy)yy' Cycle 0 0 0 100 100; sleep 6
step "5  keyboard Wave -> mouse CYCLES (nearest effect)"
call SetState 's(yyy)yy' Wave 0 0 0 60 100; sleep 6
step "6  keyboard Ripple cyan -> mouse STATIC CYAN"
call SetState 's(yyy)yy' Ripple 0 255 255 50 100; sleep 4
step "7  simulated game (10 s) -> keyboard Gaming layout, mouse ORANGE-RED; then cyan again"
gamemode-simulate-game > /dev/null 2>&1 &
sleep 3
step "   during the game: GameActive $(get GameActive)"
wait
sleep 2
step "   after the game: GameActive $(get GameActive)"
step "8  lights off -> keyboard AND mouse DARK"
call SetLightsOn b false; sleep 3
call SetLightsOn b true
step "   lights on -> cyan again"; sleep 3
step "9  mouse's own setting: Breathe blue -> mouse BREATHES BLUE, keyboard stays cyan Ripple"
call SetMouseLighting 's(yyy)yy' Breathe 0 0 255 40 100; sleep 7
step "   MouseMatch $(get MouseMatch)MouseLighting $(get MouseLighting)"
step "10 matching again, and your own state back"
call SetMouseMatch b true
eval "call SetState 's(yyy)yy' $saved"
step "done: State $(get State)MouseMatch $(get MouseMatch)"
echo "--- service log:"
journalctl --user -u g512d --since "$since" --no-pager -o cat
