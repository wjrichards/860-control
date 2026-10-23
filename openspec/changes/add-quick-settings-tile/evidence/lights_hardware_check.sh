#!/bin/sh
# Task 1.4 on the real keyboard with the installed g512d (game switch on,
# state Ripple). The user watches the keyboard; the script prints what the
# service reports and logs.
obj="io.github.g512control.Lighting1 /io/github/g512control/Lighting1 io.github.g512control.Lighting1"
get() { busctl --user get-property $obj "$@" | tr '\n' ' '; echo; }
since=$(date '+%H:%M:%S')
t0=$(date +%s)
at() { echo "[$(( $(date +%s) - t0 ))s] $*"; }

at "start: LightsOn $(get LightsOn)GameSwitch $(get GameSwitch)State $(get State)"
busctl --user call $obj SetLightsOn b false
at "SetLightsOn false: LightsOn $(get LightsOn)(keyboard should be dark)"
sleep 3

at "gamemode-simulate-game (10 s) with the lights off"
gamemode-simulate-game > /dev/null 2>&1 &
sim=$!
sleep 3
at "during the game: GameActive $(get GameActive)LightsOn $(get LightsOn)(keyboard should stay dark)"
wait "$sim"
sleep 1
at "game over: GameActive $(get GameActive)"

busctl --user call $obj SetLightsOn b true
at "SetLightsOn true: LightsOn $(get LightsOn)(Ripple should be back)"
sleep 3

busctl --user call $obj SetLightsOn b false
at "SetLightsOn false again"
sleep 3
busctl --user call $obj SetState 's(yyy)yy' Ripple 61 56 70 52 100 > /dev/null
at "SetState Ripple while dark: LightsOn $(get LightsOn)(Ripple should be back)"

echo "--- service log:"
journalctl --user -u g512d --since "$since" --no-pager -o cat
