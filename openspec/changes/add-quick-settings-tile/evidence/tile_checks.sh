# Tasks 3.3–3.5 in the nested Shell. Sourced by gnome-extension/tests/nested.sh:
#
#   toolbox run -c dev sh gnome-extension/tests/nested.sh \
#       openspec/changes/add-quick-settings-tile/evidence/tile_checks.sh OUT_DIR
#
# Prints ok/FAIL per step and leaves screenshots in OUT_DIR.

T="const t = Main.extensionManager.lookup('$uuid').stateObj?._indicator?.quickSettingsItems[0];"
TILE='[t.visible, t.checked, t.reactive, t.subtitle].join(" ")'
MARKED='t._effectItems.filter(i => i._ornament === 2).map(i => i.label.text).join(",")'
pass=0
fail=0

get() {
    shell_eval "$T String($1)" | sed -n "s/^(true, '\"\(.*\)\"')\$/\1/p"
}
act() {
    shell_eval "$T $1; 0" > /dev/null
    sleep 0.6
}
expect() {
    got=$(get "$2")
    if [ "$got" = "$3" ]; then
        echo "ok   $1"
        pass=$((pass + 1))
    else
        echo "FAIL $1: got '$got', expected '$3'"
        fail=$((fail + 1))
    fi
}
expect_call() {
    got=$(calls | tail -n 1 | cut -d' ' -f3-)
    if [ "$got" = "$2" ]; then
        echo "ok   $1"
        pass=$((pass + 1))
    else
        echo "FAIL $1: last call '$got', expected '$2'"
        fail=$((fail + 1))
    fi
}
start_fake() {
    python3 -I "$here/fake_service.py" --yes-this-is-a-private-bus >> "$out/service.log" 2>&1 &
    fake=$!
    sleep 1.5
}

echo "== 3.3 toggle"
shell_eval "Main.panel.statusArea.quickSettings.menu.open(); 0" > /dev/null
sleep 1
expect "lit: visible, checked, sensitive, effect as subtitle" "$TILE" "true true true Ripple"
screenshot 01-lit

act 't.emit("clicked", 1)'
expect_call "click sends SetLightsOn(false)" "SetLightsOn (False,)"
expect "off: unchecked, sensitive, subtitle label hidden" \
    '[t.visible, t.checked, t.reactive, t._box.get_first_child()._subtitle.visible].join(" ")' "true false true false"
screenshot 02-off

act 't.emit("clicked", 1)'
expect_call "click again sends SetLightsOn(true)" "SetLightsOn (True,)"
expect "on again" "$TILE" "true true true Ripple"

fake_set GameActive '<true>'
sleep 0.6
expect "game running: Gaming Layout" "$TILE" "true true true Gaming Layout"
screenshot 03-gaming
fake_set GameActive '<false>'

fake_set DeviceState "<'disconnected'>"
sleep 0.6
expect "disconnected: insensitive with subtitle" "$TILE" "true false false Disconnected"
screenshot 04-disconnected
fake_set DeviceState "<'permission-denied'>"
sleep 0.6
expect "no permission" "$TILE" "true false false No Permission"
fake_set DeviceState "<'connected'>"
sleep 0.6

gdbus call --session --dest io.github.g512control.Lighting1 --object-path /io/github/g512control/Lighting1 \
    --method io.github.g512control.Test.Quit > /dev/null
sleep 1
expect "service gone: tile hidden" "t.visible" "false"
screenshot 05-absent
start_fake
expect "service back: tile shown with its values" "$TILE" "true true true Ripple"

echo "== 3.4 menu"
act 't.menu.open()'
sleep 0.5
expect "header shows the device" "t.menu._headerSubtitle.text" "G512 RGB MECHANICAL GAMING KEYBOARD"
expect "effects listed" 't._effectItems.map(i => i.label.text).join(",")' "Off,Static,Breathe,Cycle,Wave,Ripple,Custom"
expect "current effect marked" "$MARKED" "Ripple"
expect "slider at the saved brightness" "Math.round(t._brightness.slider.value * 100)" "100"
expect "game switch off" "t._gameItem.state" "false"
expect "Lighting Settings item present" 'Object.keys(t.menu._settingsActions).join(",")' "io.github.g512control.Control.desktop"
screenshot 06-menu

act 't._effectItems.find(i => i.label.text === "Wave").emit("activate", null)'
expect_call "choosing Wave sends SetState with the saved colour, speed and brightness" \
    "SetState ('Wave', (61, 56, 70), 52, 100)"
expect "Wave marked" "$MARKED" "Wave"
expect "subtitle follows" "t.subtitle" "Wave"

fake_set Effects "<[('Static', true, false, 0, 0), ('Wave', false, true, 10, 90)]>"
fake_set State "<('Static', (255, 0, 0), 95, 70)>"
sleep 0.6
act 't._effectItems.find(i => i.label.text === "Wave").emit("activate", null)'
expect_call "speed clamped to the effect's range" "SetState ('Wave', (255, 0, 0), 90, 70)"
start_fake_reset() {
    gdbus call --session --dest io.github.g512control.Lighting1 --object-path /io/github/g512control/Lighting1 \
        --method io.github.g512control.Test.Quit > /dev/null
    sleep 0.5
    start_fake
}
start_fake_reset
act 't.menu.open()'

before=$(calls | wc -l)
shell_eval "$T for (let i = 0; i <= 62; i++) GLib.timeout_add(GLib.PRIORITY_DEFAULT, 16 * i, () => { \
    t._brightness.slider.value = (100 - 80 * i / 62) / 100; return GLib.SOURCE_REMOVE; }); 0" > /dev/null
sleep 2.5
calls | tail -n +$((before + 1)) | grep SetState > "$out/drag-calls.txt"
n=$(wc -l < "$out/drag-calls.txt")
min_gap=$(awk 'NR > 1 { g = $1 - prev; if (min == "" || g < min) min = g } { prev = $1 } END { printf "%.3f", min }' \
    "$out/drag-calls.txt")
last=$(tail -n 1 "$out/drag-calls.txt" | sed 's/.*, \([0-9]*\))$/\1/')
echo "     slider drag 100 -> 20 over 1 s: $n SetState calls, smallest gap ${min_gap}s, last brightness $last"
if [ "$n" -ge 2 ] && [ "$n" -le 12 ] && [ "$last" = 20 ] && awk "BEGIN { exit !($min_gap >= 0.095) }"; then
    echo "ok   slider: at most one call per 100 ms, final value sent"
    pass=$((pass + 1))
else
    echo "FAIL slider"
    fail=$((fail + 1))
fi
expect "slider shows the saved value afterwards" "Math.round(t._brightness.slider.value * 100)" "20"

act 't._gameItem.toggle()'
expect_call "game switch sends SetGameSwitch(true)" "SetGameSwitch (True,)"
expect "game switch on" "t._gameItem.state" "true"

fake_set State "<('Static', (255, 0, 0), 50, 40)>"
sleep 0.6
expect "outside change: marked effect follows" "$MARKED" "Static"
expect "outside change: slider follows" "Math.round(t._brightness.slider.value * 100)" "40"
expect "outside change: subtitle follows" "t.subtitle" "Static"
screenshot 07-menu-after-change
act 't.menu.close()'

echo "== 3.5 disable and enable"
warn_before=$(grep -c -i -E 'warn|error|critical' "$out/shell.log")
for i in 1 2 3; do
    gnome-extensions disable "$uuid"
    sleep 0.5
    gnome-extensions enable "$uuid"
    sleep 1
done
expect "tile back with current values" "$TILE" "true true true Static"
grep -i -E 'warn|error|critical' "$out/shell.log" | tail -n +$((warn_before + 1)) > "$out/new-warnings.txt"
if [ -s "$out/new-warnings.txt" ]; then
    echo "FAIL new Shell log warnings after disable/enable:"
    cat "$out/new-warnings.txt"
    fail=$((fail + 1))
else
    echo "ok   no new warnings in the Shell log"
    pass=$((pass + 1))
fi

echo "== $pass passed, $fail failed"
cp "$out/service.log" "$out/service-final.log"
[ "$fail" -eq 0 ]
