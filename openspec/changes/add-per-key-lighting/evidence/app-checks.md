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

## Still to check by hand (4.4, 4.6)

Arrow-key movement between drawn keys, drag selection, and the look of the editor in light and dark style with the accent colour. Keyboard and pointer input can't be generated over AT-SPI on this Wayland session.
