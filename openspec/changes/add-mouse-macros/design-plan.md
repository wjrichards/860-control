# Design plan: mouse macros

Planned in Penpot before any code (file "G512 Control"). This is change 7, the one `add-mouse-buttons` left macros for.

## 1. Goal

Let a G502 HERO button play a macro: a recorded sequence of keys, mouse buttons and pauses. The macro is stored on the mouse, so it keeps working on other computers and with the app closed.

## 2. Who uses it, and on what

- **Who:** the owner first, then other alpha users with a G502 HERO.
- **Where:** a GNOME desktop, mostly at the default 860 × 780 window, sometimes wide (1280) or narrow (360).
- **When:** now and then, to set up a game or a shortcut, then left alone. Macros are typed or recorded once and reused.

## 3. Inventory

| Screen or state | New or existing |
|---|---|
| Mouse › Buttons page, a button showing "Macro: <name>" in its callout | existing page, new label |
| Change dialog with a **Macro** choice and a macro picker | existing dialog, new choice |
| Mouse › **Macros** page: the macros stored on the mouse, with how many sectors are free | new page and sidebar row |
| Macros page, empty state ("No macros yet") | new |
| Macro editor: name, the steps list, how it plays (once, repeat while held, toggle) | new dialog or page |
| Macro editor, recording (keys and pauses captured as you type) | new state |
| Macro editor, a step being edited (key, mouse button, pause length) | new state |
| Errors: mouse memory full, write failed and rolled back, macro too long for a sector | new |
| Deleting a macro that buttons still use | new confirmation |
| Narrow (360) versions of the Macros page and the editor | new |
| Mouse unplugged while editing | existing behaviour (section hides), new case |

## 4. Existing pages to match

- **Buttons page and callouts:** `g512-control/src/mouse_art.rs`, `callouts.rs`; app screenshots in `openspec/changes/archive/2026-10-29-sidebar-fluid-layout/evidence/app/buttons-*.png`.
- **Change dialog:** `g512-control/src/change_dialog.rs` (mouse buttons, captured shortcut, media keys, mouse functions, Disabled). There is no screenshot yet; I'll take one from the app before drawing the After.
- **Sidebar:** the Mouse section (Lighting, DPI & Profiles, Buttons) gains a fourth row.

## 5. Design system

- The Penpot file already has the adwaita-dark and adwaita-light token sets, the illustration tokens, `Sidebar row` (Default, Hover, Pressed, Selected) and `Sidebar section header`.
- Likely new: a **Macro step row** component (key down, key up, mouse button, pause, with drag handle and delete), and a **Record** button with idle, recording and disabled states. Possibly a "sectors used" meter. No new colour tokens expected; recording uses the existing destructive red.

## 6. Hardware facts and unknowns (shape the design)

Known from the earlier spikes (`2026-10-24-add-mouse-lighting/evidence/mouse.md`):
- OnboardProfiles reports **macro format 1**, 16 user sectors of 256 bytes, 5 profiles, 11 buttons.
- Sectors 0–5 hold the directory and the five profiles. **Sectors 6–f are empty**, so macros would live there: 10 sectors, one or more macros each.
- Button entries this program can't write today (`Other`) are kept byte for byte; macro entries will be among them.

To confirm with a read-only spike, then one write behind the backup, before the boards are final:
- The button entry for a macro (expected: type `00`, then the macro's sector and offset).
- The macro command set in format 1: which of key press and release, mouse buttons, pauses, repeat-while-held, wait-for-release and end exist, and their byte layouts. libratbag's onboard-profile code is the public reference.
- Whether a macro may cross sectors, and the longest pause.

From libratbag's HID++ 2.0 driver (MIT licence; read for the format only, no code copied; `src/hidpp20.h`, `hidpp20.c`, `driver-hidpp20.c`):
- **Format id 1** is the format libratbag calls "G402", and it's the only macro format it accepts.
- **Button entry:** `00 page 00 offset`, where page is the sector and offset the byte in it.
- **Commands** (1 or 3 bytes):

  | Byte | Command | Length |
  |---|---|---|
  | `00` | no-op (one step's wait) | 1 |
  | `01` | wait for the button's release | 1 |
  | `02` | repeat while the button is held | 1 |
  | `03` | repeat until the button is pressed again | 1 |
  | `40 hi lo` | pause, milliseconds, big-endian | 3 |
  | `41` / `42` | mouse button down / up | 3 |
  | `43 mm kk` / `44 mm kk` | key press / release with modifier bits | 3 |
  | `45` / `46` | media key down / up | 3 |
  | `60 off page` | jump, which is how a macro continues in another sector | 3 |
  | `ff` | end | 1 |

- libratbag reads macros from these mice but doesn't write full ones (it only turns a single-key macro into a key binding). Writing a macro is untested in the open, so a hardware write test is still needed: one macro into sector 6, one button pointed at it, then restored.
- From the commands, the play modes could be: **play once**; **press and release parts** (wait for release); **repeat while held**; **repeat until pressed again**. The editor offers those the write test confirms.

**Safety change this needs:** today only sectors 1–5 may be written. Macros would add sectors 6–f to the writable set, still behind the backup, the CRC and the read-back check, and never the directory (0) or ROM.

## 7. Board plan and order

Pages in the "G512 Control" Penpot file, one checkpoint each:

1. **00 Brief:** goal, inventory, open questions (this plan).
2. **03 Storyboard:** Buttons → Change → Macro → pick or create → editor → record → save → the callout shows the macro. Plus: delete a macro still in use; memory full.
3. **04 Architecture:** sectors 6–f layout, the button entry, the D-Bus additions (for example `MouseMacros`, `SetMouseMacro`, `DeleteMouseMacro`) and which files change (`g512-hidpp/src/profiles.rs`, `g512d`, the app).
4. **02 Components:** Macro step row; Record button.
5. **05 Screens**, one at a time:
   - Macros page at 1280, then 860 and 360, and its empty state;
   - Macro editor, then its recording and editing states;
   - Change dialog with the Macro choice (Before and After);
   - Buttons with a macro on a callout;
   - the errors and the delete confirmation.
6. Accessibility and token audits, then exports into `evidence/design/` and the OpenSpec proposal.

## 8. Open questions (each changes the design)

1. **Mouse only?** The G512 has no G-keys and no macro storage, so keyboard macros would need the service to watch and inject keys, which only works while it runs. I suggest mouse onboard macros only.
2. **How macros are made:** record by typing (with the pauses as typed), build step by step, or both. I suggest record, then edit the list.
3. **Where they live in the app:** a **Macros** page under Mouse (a library, picked from the Change dialog) or each macro edited inside its button's Change dialog. I suggest the Macros page, because one macro can serve several buttons and profiles.
4. **Play modes:** offer only what format 1 supports (likely once, and repeat while held). Confirmed by the spike.
5. **Writing sectors 6–f:** approve adding them to the writable set, with the safeguards above.

## 9. Decisions (approved 2026-10-29)

The user approved the plan with every suggestion in §8:
1. Mouse onboard macros only; no keyboard macros.
2. Record by typing, then edit the steps list.
3. A **Macros** page under Mouse; the Change dialog picks a macro from it.
4. Play modes are whatever format 1 supports, as the spike shows.
5. Sectors 6–f join the writable set, behind the backup, the CRC and the read-back check; never the directory (sector 0) or ROM.

Storyboard approved 2026-10-29, with:
6. Deleting a macro sets the buttons that use it to Disabled: they do nothing until you choose something else.

## 10. Boards

Penpot file "G512 Control". Exports in `evidence/design/`. Ids starting `…` share the prefix `3a3cfffb-d449-803a-8008-`.

| Board | Page | Id | Export |
|---|---|---|---|
| Brief · add-mouse-macros | 00 Brief, x = 2600 | `…dcf7b19aed1f` | `00-brief.png` |
| Storyboard row 1 · Make a macro and put it on a button | 03 Storyboard, y = 5440 | `…dcf685c45b82` | `03-storyboard-make.png` |
| Storyboard row 2 · The Macros page | 03 Storyboard, y = 5958 | `…dcf73c84b6f4` | `03-storyboard-page.png` |
| Storyboard row 3 · When something gets in the way | 03 Storyboard, y = 6476 | `…dcf7576f3663` | `03-storyboard-edges.png` |
| New page **04 Architecture** (`…dd0ff0af38bc`) | | | |
| Architecture 1 · Mouse memory: sectors, button entry, a macro sector | 04 Architecture, y = 0 | `…dd1014980c08` | `04-architecture-memory.png` |
| Architecture 2 · Save and delete: app → D-Bus → g512d → g512-hidpp, write order | 04 Architecture, y = 618 | `…dd1039d8dbe2` | `04-architecture-flow.png` |
| Architecture 3 · D-Bus additions, the spike, specs touched | 04 Architecture, y = 1316 | `…dd104c2a06ca` | `04-architecture-interface.png` |
| Macro step row / Kind=Action, State=Default (component, standalone) | 02 Components, inside the states board | `…dd12b6c002ae` | `02-step-row-action.png` |
| Macro step row / Kind=Pause, State=Default (component, standalone) | 02 Components, inside the states board | `…dd12e40efb60` | `02-step-row-pause.png` |
| Macro step row · states: 10 standalone components (Action/Pause × Default, Hover, Focus, Dragging, Disabled); Hover `…dd1915814bba` `…dd1a14e77071`, Focus `…dd1a021a4083` `…dd1a15fab5b3`, Dragging `…dd1a06aa08c8` `…dd1a16e4df17`, Disabled `…dd1a09bcdd79` `…dd1a175bb078` | 02 Components, board at x = 1660, y = 480 | `…dd1a1fd7117a` | `02-step-row-states.png` |
| Record button · states: 9 standalone components (Mode=Idle × Default, Hover, Pressed, Focus, Disabled; Mode=Recording × Default, Hover, Pressed, Focus); mains Idle `…dd1cde94ffb7`, Recording `…dd1ce83a01a6` | 02 Components, board at x = 1660, y = 840 | `…dd1d36a39d9b` | `02-record-button-states.png` |
| Mouse › Macros · 1280 (sidebar row Macros selected; content in a 640 clamp: On the Mouse list + Mouse Memory meter) | 05 Screens, x = 0, y = 11400 (label above at 11352) | `…dd1de089f41a` | `05-macros-1280.png` |
| Mouse › Macros · 860 (default window; clamp fills the 556 column, meter fill 40% of 528) | 05 Screens, x = 1400, y = 11400 | `…dd1edd4c5ed8` | `05-macros-860.png` |
| Mouse › Macros · 360 (narrow page with Back; New Macro under the group labels; subtitles wrap) | 05 Screens, x = 2380, y = 11400 | `…dd1f2acedef8` | `05-macros-360.png` |
| Mouse › Macros · 860 · empty (status page: 64 px macros icon, "No Macros Yet", description, New Macro on `color.accent-strong` for AA) | 05 Screens, x = 2860, y = 11400 | `…dd1f7cb5fd3e` | `05-macros-empty-860.png` |
| Macro editor · Copy line · 860 (dialog 560: Cancel / title / Save; Name, Plays, Used by; Steps with Record + 5 step-row instances; Delete Macro) | 05 Screens, x = 3840, y = 11400 | `…dd2259b6bb22` | `05-editor-860.png` |
| Macro editor · New Macro · recording · 860 (Stop · 0:04; Name/Plays and Save dimmed; rows in Disabled state; "Waiting for the next key…" row with slot use so far) | 05 Screens, x = 4500, y = 11400 | `…dd23e3339b25` | `05-editor-recording-860.png` |
| Macro editor · Copy line · editing steps · 860 (pause row in Focus at 120 ms; a Click · Mouse button · Left step; Add Step row with its menu open: Key…, Mouse Button…, Media Key…, Pause) | 05 Screens, x = 5160, y = 11400 | `…dd24280a3fbf` | `05-editor-editing-860.png` |
| Change dialog · G9 · Before (420 × 600 dialog rebuilt from `change_dialog.rs`; the live capture of the open dialog failed, the test window was covered, so `05-buttons-before-app.png` is the live Buttons page for style) | 05 Screens, x = 0, y = 12400 | `…dd24d167f564` | `05-change-dialog-before.png` |
| Change dialog · G9 · After (new Macro group after Keyboard: radio + "Stored on the mouse" + picker "Copy line ⌄"; picker menu open: the macros, then New Macro…; Apply enabled) | 05 Screens, x = 520, y = 12400 | `…dd25978925f8` | `05-change-dialog-after.png` |
| Mouse › Buttons · 860 · Before (app) (live capture of main 7b710b4 with the fake service: G9 is callout 6, "Next profile") | 05 Screens, x = 0, y = 13200 | `…dd261d08e54b` | `05-buttons-macro-before.png` |
| Mouse › Buttons · 860 · After (same capture; G9's callout reads "Macro: Copy line", patched in place over the card at #333336) | 05 Screens, x = 960, y = 13200 | `…dd261d327ce9` | `05-buttons-macro-after.png` |
| Macro editor · error · too long · 860 ("Long combo", 47 steps; AdwBanner: "This macro needs 112% of a slot. Remove some steps to save it."; Save dimmed) | 05 Screens, x = 0, y = 14200 | `…dd269a889606` | `05-error-too-long.png` |
| Macro editor · error · write failed · 860 (AdwBanner with Try Again; Save stays on) | 05 Screens, x = 620, y = 14200 | `…dd269af5496b` | `05-error-write-failed.png` |
| Macro editor · error · mouse unplugged · 860 (AdwBanner; Save and Delete Macro dimmed; Record still works) | 05 Screens, x = 1240, y = 14200 | `…dd269b2fe7db` | `05-error-unplugged.png` |
| Delete confirmation · Copy line (AdwAlertDialog 372: names G9 in profile 4, "does nothing until you choose something else"; Cancel / Delete) | 05 Screens, x = 1860, y = 14200 | `…dd26a5197188` | `05-delete-confirmation.png` |
| Mouse › Macros · 860 · memory full (full scroll, 900 tall; 10 of 10 · Full; New Macro dimmed; "Delete a macro you don't use to make room for a new one.") | 05 Screens, x = 2340, y = 14200 | `…dd26eb0df251` | `05-error-memory-full.png` |

Tokens added 2026-10-29 (both sets): `color.focus-ring`, `color.bg.keycap`, `color.border.keycap`, `color.shadow.drag`. Architecture approved with A: one macro per sector (storyboard row 3 frame 2 now "Too long, or memory full").

Step counting (2026-10-29): a step is one row in the editor (a press-and-release is one step, two commands). Slot use is shown as a percentage ("uses 10% of its slot"); a slot holds about 40 key presses. List subtitles, the memory note and the storyboard's too-long frame were updated to match.
Recording captures keys only (approved 2026-10-29); mouse-button and media-key steps are added through Add Step.
Errors use GNOME's own widgets (AdwBanner in the existing `color.bg.banner`, AdwAlertDialog), not the storyboard's amber and red panels; memory full moved from the editor to the Macros page (New Macro dimmed), so nobody records a macro that can't be saved.
