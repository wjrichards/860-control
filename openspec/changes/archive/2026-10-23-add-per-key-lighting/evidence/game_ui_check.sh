#!/bin/sh
# Gaming map view, then the game switch and banner during a simulated game.
drive=/tmp/claude-1000/-var-home-wrichards-Projects-Software/b0faba46-855d-40b2-82f0-5aa4f5cb7481/scratchpad/a11y_drive.py
out=/tmp/claude-1000/-var-home-wrichards-Projects-Software/b0faba46-855d-40b2-82f0-5aa4f5cb7481/scratchpad/per-key/game-monitor.txt
prop() {
    busctl --user get-property io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \
        io.github.g512control.Lighting1 "$1"
}
echo "--- Gaming map in the editor:"
python3 -I "$drive" clickrole "toggle button" "Gaming"
sleep 0.5
python3 -I "$drive" dump | grep -E 'toggle button: (Gaming|W|Escape|Up|Space),' | sed 's/^ *//'
python3 -I "$drive" dump | grep -E 'toggle button: Gaming' | sed 's/^ *//'
python3 -I "$drive" clickrole "toggle button" "Custom"
python3 -I "$drive" clickrole button "Back"
sleep 0.7

timeout -s INT 20 stdbuf -oL busctl --user monitor \
    --match "type='method_call',interface='io.github.g512control.Lighting1',member='SetGameSwitch'" > "$out" 2>&1 &
sleep 0.5
echo "--- switch on through the app:"
python3 -I "$drive" clickrole switch "Switch to Gaming Layout While a Game Runs"
sleep 0.7
echo "GameSwitch: $(prop GameSwitch)"
echo "--- simulated game:"
gamemode-simulate-game &
sleep 2
echo "GameActive: $(prop GameActive)"
python3 -I "$drive" dump | grep -i 'gaming layout active' | sed 's/^ *//'
wait
sleep 1
echo "--- after the game:"
echo "GameActive: $(prop GameActive)"
python3 -I "$drive" dump | grep -ci 'gaming layout active'
python3 -I "$drive" clickrole switch "Switch to Gaming Layout While a Game Runs"
sleep 0.7
echo "GameSwitch: $(prop GameSwitch)"
sleep 1
grep -A1 'Member=SetGameSwitch' "$out" | grep -o 'BOOLEAN [a-z]*'
