# Mouse illustration and callouts (tasks 0.2, 4.1, 4.2, 5.1, 5.2)

**How it was checked, on 2026-10-28:** the same setup as `app-views.md`:
- the worktree's debug build ran inside the installed Flatpak (`run_app_fake.sh`);
- it talked to `fake_service.py` under `io.github.g512control.Lighting1Test`;
- g512d was not touched.

`run_app_fake.sh` now takes two more settings:
- `SIZE`: sizes the window before the screenshot (`G512_CONTROL_SIZE`);
- `PRE`: a call made on the fake before the app starts, e.g. `sh fake_call.sh SetState Static "(38, 162, 105)" 50 100`.

## 0.2 The SVGs

`cmp` shows that `g512-control/data/g502-top-ink.svg` and `g502-side-ink.svg` are byte-identical to the user's files in `~/Pictures/` and to `design/mouse-art/`. `mouse_ink.rs` embeds them with `include_str!`.

## 4.1 Parser

These `mouse_ink` unit tests pass:
- **`both_files_parse_into_their_layers`:**
  - every `d` in both files parses;
  - the top view has 52 outline, 3 LED, 1 logo and 2 glow paths;
  - the side view has 41, 3, 1 and 2.
- **`path_commands`:** M, L, C and Z are read; Q and short paths are errors.
- **`unknown_class_is_an_error`.**

The G cut is drawn by `trace_g_top` and `trace_g_side` in `mouse_art.rs`.

## 4.2 Illustration

**Unit tests (`mouse_art`):**
- `every_badge_hits_its_own_button`: the hit test maps each badge centre to its own button. The card's corner hits nothing.
- `badges_and_buttons_are_one_to_one`: badges 1–11 map onto service buttons 0–10.
- `badges_stay_on_the_card`.

**Unit test (`art`):** `mouse_follows_the_service_rules`. The drawing's colour follows g512d's `mouse.rs`:
- its own setting while not matching;
- dark while the keyboard's lights are off;
- while matching, Wave becomes Cycle and Ripple becomes Static;
- a key map becomes Static in its W key's colour.

**Screenshots, compared with the boards:**

| Screenshot | Board | Result |
|---|---|---|
| `app/mouse-view.png` (860×640, keyboard on Static #26A269, mouse matching) | `design/05-after-mouse.png` | Same layout: green ink, lit G logo and DPI LEDs with glow, the 11 badges on their buttons, the hint under the side view, callouts 1–6 and 7–11 in two columns to the right. The drawing is a little smaller, because the card's padding takes room the board gave to the drawing. |
| `app/mouse-narrow.png` (360×800) | `design/05-narrow-mouse.png` | Same: both views on top, the callouts in pairs below, no hint, the switcher in the bottom bar. |
| `app/mouse-off.png` (lights off) | the Off variant | Dim white outlines, no glow, grey logo and LEDs. Badges and callouts are unchanged. |

**Width change:** to fit the callouts beside the drawing, each view's column is now clamped to its board width: 694 px for the keyboard and 760 px for the mouse. Before, `AdwPreferencesPage` clamped both to 600 px. `app/keyboard-view.png` was retaken at the new width, and the keyboard card is now 694 px wide, as on its board. This settles the clamp difference noted for 7.3.

## 5.1 Callouts and the shared Change dialog

`ChangeDialog` moved to `change_dialog.rs`. The rows, the callouts and the drawing all open it through `ButtonSection::open_dialog`.

**`a11y_callouts.py`, after switching to the Mouse view:**
- The drawing is an `image` named "G502 HERO mouse, Static, dim purple" (the fake's default Ripple #3D3846, matched as Static).
- There are 11 callout buttons. Each is named with its number, button and action, e.g. "6 G9, Next profile".
- "1 Left Click, Left click · fixed" has `sensitive=False` and no description.
- The other 10 are sensitive, and each has the description "Change what <button> does".
- Activating "6 G9" opens a dialog named "G9", with "Next profile" checked. Cancel closes it.

AT-SPI can't click inside a custom-drawn area, so the pointer click on the drawing wasn't driven here:
- `every_badge_hits_its_own_button` covers which button a click lands on;
- the click handler calls the same `open_dialog(index)` as the callout;
- the click itself is left for the hands-on pass, under 7.4's "remap a button from the drawing".

## 5.2 Callouts follow changes

`check_callouts_follow.py` changes the fake's `MouseButtons` and `MouseActiveProfile` over its Test interface, as a G9 press or another client would. It reads G9's callout after each step:

```
profile 4, G9: ['6 G9, Next profile']
switched to profile 2 (as G9 on the mouse), G9: ['6 G9, Ctrl+C']
profile 2's G9 changed to Volume up, G9: ['6 G9, Volume up']
back to profile 4, G9: ['6 G9, Next profile']
```

The app wasn't restarted between steps.
