# Design review: built screens and the Penpot boards (task 7.3)

**Date:** 2026-10-28.

**Method:** the board exports in `design/` were compared with app screenshots. The screenshots are from the worktree build running against the fake service, at 860×640 and 360×800 (`run_app_fake.sh` with `SHOT`, `VIEW` and `SIZE`). The penpot MCP server wasn't connected this session, so the review used the exports, which are the approved versions.

**Side-by-side sheets:** board on the left, app on the right.
- `app/compare-keyboard.png`
- `app/compare-narrow.png`: keyboard board, keyboard app, mouse board, mouse app.
- `app/compare-states.png`: rows for keyboard missing, mouse missing and mouse no access.

## Fixed during the build

| Board | Was | Now |
|---|---|---|
| All screens | `AdwPreferencesPage` clamped every view to 600 px, so the keyboard card was about 570 px wide and the mouse callouts didn't fit beside the drawing. | Each view is clamped to its board width: 694 px for the keyboard and 760 px for the mouse (design.md D1). |
| `05-after-mouse` | The hint sat under the drawing, and the callouts were too tall to sit beside it. | The hint sits under the side view, and the callouts are compact chips in two columns (1–6 and 7–11) beside the drawing, as on the board. |
| `05-narrow-mouse` | — | Both views on top, the callouts in pairs below, and no hint, as on the board. |

## Matches

- **`05-after-keyboard` / `app/keyboard-view.png` (Static green):** the card, the case, the key tiles, the caption, and the Device, Lighting and Games groups.
- **`05-after-mouse` / `app/mouse-view.png`:**
  - green ink, with the lit logo and DPI LEDs and the glow;
  - badges 1–11 on the same buttons;
  - Left Click's dashed, insensitive chip;
  - the switcher in the header.
- **`05-narrow-keyboard`, `05-narrow-mouse`:** the bottom switcher bar, the window title, and the illustration scaled to the column.
- **`05-keyboard-missing`, `05-mouse-missing`, `05-mouse-no-access`:** the status page sits in the device's own view, the other view keeps working, and the titles match. The no-access command writes both rule lines.
- **`04-motion`:** see `motion.md`. Ripple, Wave and Breathe follow the preview's timings, which `motion.rs`'s tests check against `render.py`.

## Differences left in place, for the user to decide

1. **Group titles.** The boards title the groups by what they are: Device, Lighting, Profile, Buttons. The app keeps its older titles: "Keyboard" and "Mouse" for the device groups, plus "Mouse Profile" and "Mouse Buttons". Now that each view is named for its device, the prefix repeats the view's name.
   - Renaming is a one-line change per group.
   - But the main spec names a "Mouse Buttons group" (from add-mouse-buttons), and the README uses the same names.
   - **Recommendation:** rename to match the boards, with a spec delta for "Mouse Buttons". Not done here without the user's say.
2. **The mouse drawing is a little smaller** than on the board (about 440 px wide against 480 px), because the card's 16 px padding comes out of the same 760 px. It's not worth shrinking the callouts to win it back.
3. **The narrow keyboard caption** says "Click the keyboard to edit Key Maps" where the narrow board says "Tap". It's a desktop app, so it stays "Click", and the wide and narrow captions are the same.
4. **The narrow board leaves out the subtitle** of the Edit Key Maps row. The app keeps it at every width, because it explains what the page does.
5. **Status page icons.** The boards drew outline icons: a keyboard, and a mouse for both "not connected" and "no access". The app uses GNOME's symbolic icons:
   - `input-keyboard-symbolic` and `input-mouse-symbolic` for "not connected";
   - `dialog-password-symbolic` (a key) for "no access", which says "permission" more plainly.
   These follow the HIG and the user's icon theme.
6. **Status page wording.** The boards say "The Mouse tab keeps working"; the app says "view", to match the README and the switcher's accessible names.
7. **The no-access command shows as selected** when the page opens. The label is selectable, and GTK selects its text when it gets the first focus. That lets a keyboard user press Ctrl+C straight away. This was already the case before this change (the same label is on main).
8. **The command block** shows one rule per line, at full width, where the board wraps it narrower. The text is the same.

## Open checks (task 7.4)

`motion.md` lists them: G9's hover and confirm pulse need a real pointer and Apply, and the animations-off check uses the desktop's setting.
