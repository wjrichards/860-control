# App checks (task group 4), 2026-10-22

The Flatpak was rebuilt from the worktree and installed, then started with `gio launch` on its desktop file. It was driven over AT-SPI with `a11y_drive.py`, and the bus was watched with `busctl --user monitor`.

## 4.2 Rows per effect

`rows_per_effect.sh` sets each effect from outside and lists the rows the app shows:

```
Off: Brightness,Key Maps
Static: Colour,Brightness,Key Maps
Breathe: Colour,Speed,Brightness,Key Maps
Cycle: Speed,Brightness,Key Maps
Wave: Speed,Brightness,Key Maps
Ripple: Colour,Speed,Brightness,Key Maps
Custom: Brightness,Key Maps
```

## 4.3 Editor

`editor_check.sh`:
- The editor page has the Custom/Gaming toggles, the six group buttons and 103 key toggles (all addressable keys except Right Super, which the G512 doesn't have).
- **WASD** selects exactly W, A, S and D.
- **Paint Selected** sent one `SetKeys "custom"` with exactly those four keys.

**The paint rate is limited by GTK as well.** A `GtkButton` ignores a new activation for 250 ms while it shows as pressed:
- `burst_check.sh`: 10 clicks over 207 ms gave **1** call, and 6 clicks 300 ms apart gave **6** calls, 300 ms apart.
- So painting with the button can't go above about 4 edits per second, under the 100 ms limit. The `Throttle` stays in the path, and its unit tests cover holding and flushing.

`gaming_view_check.sh`: with the Gaming map selected, Escape reads "grey" and W, Up and Space read "red", which is the default Gaming map. Switching back to Custom shows the Custom map.

## 4.5 Game switch and banner

`game_ui_check.sh`:
- Turning the switch on in the app sent `SetGameSwitch true`, and the service's `GameSwitch` became true.
- During `gamemode-simulate-game`, `GameActive` was true and the app showed the banner "Gaming layout active while a game runs".
- After the game, `GameActive` was false and the banner was gone.
- Turning the switch off set `GameSwitch` back to false.

## Bugs found and fixed during these checks

1. **The Key Maps row had no accessibility action**, so it couldn't be activated except with a pointer or Enter. It now uses a real button as its activatable widget. The button takes the row's name, "Key Maps".
2. **Keys were announced as just their label** ("W"). A `GtkButton` with a label is named by a "labelled by" relation, which takes precedence over the accessible label. The relation is now removed, and keys read "W, red", "Escape, off" and so on.
3. **The banner was in the accessibility tree while hidden.** It is now also made invisible when not revealed.
4. **Clicking Gaming showed the Custom colours.** Custom's "toggled" fired before Gaming was active. Both toggles now redraw when they become active.

## 4.6 Hands-on pass with the user

- **Custom map on the keyboard:** the user painted the Custom map in the editor with the Custom effect chosen and saw it on the keyboard ("editor pass works, custom shows on the keyboard").
- **Editing Gaming while Ripple shows:** the keyboard stayed on Ripple. The user selected E in the Gaming view, painted it green and saw it in the drawing (screenshot). The service saved `08 = "#26a269"` under `[gaming]` in `state.toml` at 06:04:59, and `[custom]` was unchanged (W, A, S and D).
- **Style:** the editor follows light and dark style and the accent colour ("both looked right").

**A missing paint was a missing selection, not a bug.** Two earlier Gaming paints (Q, then E) never reached the service:
- `state.toml` didn't change, and recordings of calls and error replies were empty.
- For E, the screenshot and the accessibility tree showed E unselected, and only Q, already red, outlined from an earlier scripted test. Paint Selected repainted Q, so nothing visibly changed.
- After Clear Selection and a single click, the tree showed E selected, and the paint went through.

## 4.4 Keyboard movement and selection

The editor adds no key handling of its own. GTK's directional focus moves between the drawn keys, because they are positioned to match the keyboard, and Space toggles the focused key.

The task asked for an AT-SPI pass as well, but keyboard input can't be generated over AT-SPI on this Wayland session. So the user's hands-on pass is the evidence. In the Gaming view, after clicking E:
- **→** moved focus to R, and **↓** moved it to F. Space selected each one.
- **↑** from F returned to R.
- At the end, the accessibility tree showed E, R and F selected.
- Painting the selection uses the same Paint Selected button as the 4.6 pass.

**A held Space toggles twice.** On the first try, R ended up unselected. With this desktop's key repeat (500 ms delay, 30 ms interval), holding Space long enough starts repeat activations. GTK lets a second toggle through after its 250 ms activation guard. A quick tap selected R as expected. This is standard GTK toggle-button behaviour, not something the editor adds.
