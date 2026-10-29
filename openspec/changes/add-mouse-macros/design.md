# Design: mouse macros

## Context

See `proposal.md` for why.

**What exists today:**
- `g512-hidpp/src/profiles.rs` reads and writes format-2 profile sectors, through `UserSector`, which can only be built for sectors 1 to the profile count.
- Button entries it can't decode are `ButtonAction::Other([u8; 4])` and are never written.
- g512d backs up sectors 0–5 once before the first write (`profile_backup::save_once`), then edits profiles through `Controller::edit_mouse_profile`.
- The app's Change dialog (`change_dialog.rs`) offers mouse buttons, a captured shortcut, media keys, mouse functions and Disabled.

**What the mouse reports (spike 6):**
- 16 sectors of 256 bytes;
- 5 profiles;
- macro format 1;
- sectors 6–f empty.

The macro format and the button entry come from libratbag's HID++ 2.0 driver (MIT; read for the format, no code copied). No open project writes full macros, so the hardware spike in tasks 1.x confirms them before anything else is built on them.

**Design boards:** Penpot file "G512 Control". The 04 Architecture boards cover the memory map, the save and delete order, and D-Bus. The 05 Screens boards are listed in `design-plan.md` §10; exports and `audit.md` are in `evidence/design/`.

## Goals / Non-Goals

**Goals:**
- A macro on a mouse button that plays from the mouse's own memory.
- No write that could leave a button pointing at an empty sector, or touch the directory or ROM.
- Everything recorded or edited in the app; the service only validates and writes.

**Non-Goals:**
- Keyboard (G512) macros. The G512 has no macro memory; that would need a running daemon injecting keys.
- Macros longer than one sector (the `60` jump command).
- Recording mouse clicks, or recording outside the app window.
- Editing a macro whose commands the app can't decode. It is shown as used and can only be deleted.

## Decisions

### One macro per sector, at offset 0
Each macro owns a whole sector (6–f, so ten macros), and its button entry is `00 ss 00 00`.
- **Why:** writes stay whole-sector with a CRC and read-back, exactly like profiles, and freeing a macro is clearing one sector.
- **Alternative:** pack several macros per sector, or chain sectors with `60`. Either way one save would rewrite another macro's bytes, and `60` is untested on this mouse.
- **Cost:** a slot holds about 40 key presses (each is a down and an up, 6 bytes), which is plenty for the shortcuts and combos this is for.

### `MacroSector` mirrors `UserSector`
A newtype that can only be made for sectors above the profile count and below the sector count. `write_sector` takes either kind, through a small trait or enum, so the "never 0, never ROM" rule stays in the type and not in callers.
- **Alternative:** a range check at each call site. Rejected, because one missed check would be a write to the directory.

### Step model
- **On the wire:** the bus and g512-hidpp carry steps as their commands (key-down, key-up, button-down, button-up, media-down, media-up, pause), so the service never guesses what the user meant.
- **In the editor:** the app pairs a down with its matching up into one "Press and release" row. A down whose up doesn't follow directly (a held key around other steps) shows as its own "Hold down" and "Let go" rows.
- **Step count and slot use:** "N steps" counts editor rows. "Uses P% of its slot" is the encoded size over 254.

### Names in g512d's state file
The mouse has nowhere to put a name. `store.rs` keeps `macro_names: {sector → (sha256 of the sector's macro bytes, name)}`. A name whose hash no longer matches is dropped, so a macro changed on another computer shows as "Macro in slot N" instead of a wrong name.
- **Alternative:** store the name inside the sector after `ff`. Rejected, because other software may treat those bytes as free.

### Save and delete order
- **Save:**
  1. Validate.
  2. Pick the lowest free sector (or the macro's own sector).
  3. Back up 6–f once to `mouse-macros-backup.txt` (same file format as the profile backup, so `--restore-mouse-profiles` reads it).
  4. Write and read back.
  5. On a mismatch, write the old bytes back.
  6. Store the name.
  7. Return the sector.

  Pointing a button at it is a separate `SetMouseButton` call, the same profile write as today. If that call fails, the macro is just unused.
- **Delete:**
  1. Set every button that plays the macro to Disabled, profile by profile.
  2. Only then clear the sector.

  So no button ever points at a free sector.

### D-Bus shape
- `MouseMacros a(yssa(sqy))`: sector, name, mode, steps.
- `MouseMacroSectors (yy)`: used, total.
- `SaveMouseMacro(y sector, s name, s mode, a(sqy) steps) → y`, where sector 0 means a new macro.
- `DeleteMouseMacro(y)`.
- `SetMouseButton` gets kind `"macro"` with value = sector.
- **Errors:** today's calls only use `InvalidArgs` and `Failed`. The new ones get names (`io.github.g512control.Error.MacroTooLong`, `.MacroMemoryFull`, `.WriteFailed`) so the app can show the right banner without matching text.

### App structure
- **New files:**
  - `macros_page.rs`: list, meter and empty state, in a 640 px clamp;
  - `macro_editor.rs`: an `adw::Dialog` with a ToolbarView, a banner, the settings group, a steps `ListBox` with drag-and-drop, and Add Step as a `gtk::MenuButton`.
- **Recording:** an `EventControllerKey` on the dialog in the capture phase (the Change dialog's shortcut capture works the same way). Timestamps come from the key events, so pauses are the real gaps.
- **Change dialog:** a fourth group "Macro" after Keyboard, a radio row with a `gtk::DropDown` of macros plus "New Macro…".
- **Colours:** errors use `adw::Banner` and deletion `adw::AlertDialog`. No new colours are needed; the `color.bg.banner` token mirrors libadwaita's banner.

### Play modes after the spike
The editor's Plays drop-down shows only modes the write test confirmed. The spec names all of them but gates each on confirmation, the same pattern media keys followed in change 6.

## Risks / Trade-offs

- [The mouse rejects or mis-plays a macro written this way] → Spike first (tasks 1.x), with a spare profile and sector 6 only, restored from the backup afterwards. If it fails, stop and rethink before any other task.
- [A save is interrupted between the macro write and the button write] → The macro is unused but harmless. It shows on the Macros page and can be deleted.
- [The state file and the mouse disagree (names lost, mouse used elsewhere)] → The hash check drops stale names, and the app falls back to "Macro in slot N".
- [Ten slots and about 40 keys each feel small] → "Too long" is caught in the editor before Save, with the percentage shown all the time. Chaining with `60` can come later behind its own spike.
- [The focus ring is under 3:1 on cards (audit, Medium)] → It is libadwaita's own ring. It's recorded in `audit.md`, and changing it is a separate decision.

## Migration Plan

- No settings migration. The state file gains an optional `macro_names` table; older services ignore it.
- An older app with a newer service: it shows a macro button as "Set elsewhere (kept as it is)", today's text for unknown kinds, and never writes it.
- A newer app with an older service: no `MouseMacros` property, so no Macros row and no Macro choice.
- **Rollback:** `g512d --restore-mouse-profiles mouse-macros-backup.txt` puts sectors 6–f back. Then restore the profile backup to clear any button entries that point at macros.
