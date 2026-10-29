# Design review: macros, app against the boards

2026-10-29. Each screenshot is the debug build (worktree `worktree-mouse-macros`) run against `gnome-extension/tests/fake_service.py`, seeded with the boards' four macros and G9 playing Copy line in profile 4. The boards are the exports in `../design/`, ids in `design-plan.md` §10.

The hooks that opened each state:
- `G512_CONTROL_VIEW=macros` for the Macros page;
- `G512_CONTROL_MACRO=6|new` with `G512_CONTROL_MACRO_STATE` for the editor;
- `G512_CONTROL_BUTTON=8` for G9's Change dialog;
- `Test.Set` before start for the empty, full, too-long and unplugged data.

| Board | Screenshot | Result |
|---|---|---|
| Mouse › Macros · 1280 | `macros-1280.png` | Matches: 640 clamp, "On the Mouse" and its description, New Macro beside the title, four rows with the boards' subtitles, Mouse Memory "4 of 10 macro slots used · 6 free" and its note. |
| Mouse › Macros · 860 | `macros-860.png` | Matches; the column fills the space between the 32 px margins. |
| Mouse › Macros · 360 | `macros-360.png` | Matches: Back, New Macro under the group title, subtitles wrapping. |
| Mouse › Macros · 860 · empty | `macros-empty.png` | Matches: icon, "No Macros Yet", the keys-and-pauses text, New Macro as a suggested pill. |
| Mouse › Macros · memory full | `macros-full.png` | Matches: New Macro insensitive, "Full", the delete-a-macro note. |
| Macro editor · Copy line · 860 | `editor.png` | Matches: Cancel / title / Save, Name, Plays, Used by "G9 in profile 4", "5 steps · uses 10% of its slot · drag to reorder", the five steps, Add Step, Delete Macro. |
| Macro editor · recording | `editor-recording.png` | Matches: "New Macro", Stop · 0:01, the heard steps dimmed (Home, Pause 312 ms, Shift+End), "Waiting for the next key…" with "uses 6% so far", Name, Plays and Save off. |
| Macro editor · editing steps | `editor.png` | Same controls; the open Add Step menu and the focused pause weren't captured. |
| Error · too long | `editor-too-long.png` | Matches: the banner "This macro needs 102% of a slot…", "needs 102% of its slot", Save off. |
| Error · write failed | `editor-write-failed.png` | Matches: the banner with Try Again; Save stays on. libadwaita wraps the banner's text above its button at this width. |
| Error · mouse unplugged | `editor-unplugged.png` | Matches: the banner, Save and Delete Macro off, the steps kept. The Mouse section leaves the sidebar, as for every mouse page. |
| Delete confirmation | `editor-delete.png` | Matches: "Delete “Copy line”?", the G9-in-profile-4 sentence, Cancel and a destructive Delete. |
| Change dialog · G9 · After | `change-dialog.png` | Matches: the Macro group after Keyboard, the radio with "Stored on the mouse", the picker on Copy line. G9's callout behind reads "Macro: Copy line". |
| Buttons · After (macro on G9) | `change-dialog.png` (behind the dialog) | Matches: callout 6 G9 reads "Macro: Copy line". |

## Differences, all explained

- **Icon:** the boards' list-and-play icon ships as `g512-macros-symbolic`, a GResource; Adwaita's nearest one was two arrows.
- **Add Step menu:** Mouse Button and Media Key are submenus, not "…" items that open dialogs.
- **Colours:** buttons use libadwaita's own styles (suggested-action, destructive-action). The Record dot is red through the `error` class. The boards' raw values were stand-ins for these.
- **Fake data:** "Sprint toggle" shows "1 step"; the board said 2. The fake holds one Shift press and release, which is one step under the approved counting rule.

## Accessibility

Over AT-SPI, with the Copy line editor open:
- "Move step": 10 (each menu button reports itself and its toggle);
- "Delete step": 5;
- "Pause in milliseconds": 2;
- Record, Add Step and Delete Macro are all present.

Rows rebuilt after an edit are left for the hands-on check (task 5.3).
