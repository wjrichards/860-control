# Tasks

Boards are in the Penpot file "G512 Control" (ids in `design-plan.md` §10); exports are in `evidence/design/`.

## 1. Hardware spike (with the user, before any other task)

- [ ] 1.1 Read only: dump sectors 0–f with `memoryRead` and record in `evidence/hardware.md`, for each of sectors 6–f:
  - whether it is all `ff`;
  - what its last two bytes hold.

  Done when the file lists all 16 sectors and states whether 6–f are free.
- [ ] 1.2 One write, with the user watching:
  - **Backup:** back up sectors 1–f to a file.
  - **Macro:** write a two-key macro (`a`, then `b`, played once) to sector 6, with `ff` padding and a CRC.
  - **Button:** set profile 5's entry 8 (G9) to `00 06 00 00`.
  - **Check:** the user switches to profile 5 and presses G9 in a text box.

  Done when `evidence/hardware.md` records what was typed and both writes read back exactly.
- [ ] 1.3 With the same setup, try each play mode in turn (`01`, `02`, `03` as the first byte) and one 2000 ms pause, rewriting sector 6 each time. Done when `evidence/hardware.md` says, for each mode, what the user saw.
- [ ] 1.4 Restore sector 6 and profile 5 from the 1.2 backup. Done when both read back byte for byte equal to the backup.
- [ ] 1.5 Record the result in `design.md`: which play modes the editor offers, and anything about the format that differs from the spec. If the mouse didn't play the macro at all, stop and report before going on.

## 2. Protocol (g512-hidpp)

- [ ] 2.1 Add `macros.rs` with `Step`, `PlayMode`, `encode`, `decode` and the 254-byte limit. Verify with unit tests:
  - Copy line encodes to the exact bytes in the "Encode Copy line" scenario;
  - decode(encode(x)) == x;
  - a 255-byte macro is refused;
  - unknown commands decode as "unknown".
- [ ] 2.2 Add `MacroSector` (only sectors above the profile count and below the sector count) and let `write_sector` take it. Verify with tests that sectors 0, 1–5, `0x10` and `0x0101` are refused before any request is sent.
- [ ] 2.3 Add `ButtonAction::Macro { sector, offset }`, decoded from `00 ss 00 oo` and encoded only for offset 0 of a macro sector. Verify with a test that "Point a button at a macro" changes only bytes `0x40`–`0x43` and the CRC.
- [ ] 2.4 Give the fake mouse in `fake.rs` writable sectors 6–f, starting as all `ff`. Verify with a test that writes and reads back a macro sector.

## 3. Service (g512d)

- [ ] 3.1 Read sectors 6–f when the mouse connects and offer `MouseMacros` and `MouseMacroSectors`. Unknown or unreadable macros count as used. Verify with a controller test on the fake mouse: two macros give "2 of 10".
- [ ] 3.2 Keep macro names in the state file as `{sector: (sha256, name)}`, and drop a name whose hash no longer matches. Verify with a store test where the sector changes behind the service's back.
- [ ] 3.3 Back up sectors 6–f once to `mouse-macros-backup.txt` before the first macro write, and make `--restore-mouse-profiles` accept that file. Verify with a test that a second save leaves the backup untouched, and that restore puts the sectors back.
- [ ] 3.4 Implement `SaveMouseMacro`:
  - validate the steps;
  - pick the lowest free sector;
  - write and read back;
  - on a mismatch, write the old bytes back;
  - return the sector.

  Verify with tests for the named errors `io.github.g512control.Error.MacroTooLong`, `.MacroMemoryFull` and `.WriteFailed` (fake read-back mismatch: sector unchanged afterwards).
- [ ] 3.5 Implement `DeleteMouseMacro`: set every button that plays the macro to Disabled in every profile, then clear the sector and drop the name. If a profile write fails, stop and keep the macro. Verify with tests for "Delete a macro in use" and for a failing profile write.
- [ ] 3.6 Add the kind `macro` to `buttons.rs` (value = sector), and refuse a sector that holds no macro. Verify with the "G9 plays a macro" and "Macro sector that is empty" scenarios as tests.
- [ ] 3.7 Add the members to `dbus.rs`, and the same members to `gnome-extension/tests/fake_service.py`. Verify with `busctl --user introspect` on both, and with a test that an existing v1 call still works.

## 4. App

- [ ] 4.1 Make `service.rs` read `MouseMacros` and `MouseMacroSectors`, treating them as optional (an older service has neither). Verify with parse tests, including "Service without macros".
- [ ] 4.2 Add a Macros page id and sidebar row under Mouse, using a list-and-play icon like board `…dd1de089f41a`'s sidebar row (ship it as a symbolic icon resource if Adwaita has none). Show the row only while macros are offered. Verify with a screenshot (`G512_CONTROL_VIEW=macros`) against `05-macros-1280.png`.
- [ ] 4.3 Build `macros_page.rs` per boards `05-macros-1280.png`, `-860.png` and `-360.png`:
  - a 640 px clamp;
  - the group "On the Mouse" with "They work on other computers too, even with this app closed" and New Macro;
  - rows with subtitles like "5 steps · Plays once · G9 in profile 4", "Not on a button", and "Made on another computer" for unnamed macros, titled "Macro in slot N";
  - "Mouse Memory" with "N of 10 macro slots used", "N free" or "Full", and "Each macro takes one slot. The editor shows how much of it a macro uses."

  Verify with screenshots at 1280, 860 and 360 against the boards.
- [ ] 4.4 Add the empty state (`05-macros-empty-860.png`):
  - "No Macros Yet";
  - "Record keys and pauses, then put the macro on a mouse button. Macros are stored on the mouse, so they work anywhere.";
  - a suggested New Macro button;
  - no meter.

  Add the full state (`05-error-memory-full.png`): New Macro insensitive and "Delete a macro you don't use to make room for a new one." Verify with fake-service screenshots of both.
- [ ] 4.5 Build `macro_editor.rs`, a 560-wide `adw::Dialog` per `05-editor-860.png`:
  - Cancel, the title and Save;
  - Name, Plays (only the modes confirmed in 1.5) and Used by;
  - "Steps" with "N steps · uses P% of its slot · drag to reorder";
  - step rows per `02-step-row-states.png`, with keycaps, a pause spin (ms) and a delete button;
  - accessible names "Move step" and "Delete step".

  Verify with a screenshot against the board, and an AT-SPI check of the names.
- [ ] 4.6 Add recording per `05-editor-recording-860.png`:
  - "Stop · 0:04";
  - keys from the dialog's key events, with real gaps as pauses and no clicks;
  - the "Waiting for the next key…" row with "uses N% so far";
  - the other controls insensitive;
  - Cancel discards;
  - Record on an existing macro appends.

  Verify with a test that feeds key events with timestamps, then a hands-on check.
- [ ] 4.7 Add editing per `05-editor-editing-860.png`:
  - drag to reorder;
  - a keycap opens the shortcut capture;
  - the pause spin;
  - Add Step with "Key…", "Mouse Button…", "Media Key…" and "Pause" (50 ms);
  - press-and-release pairing, with "Hold down" and "Let go" for held keys.

  Verify with unit tests for the pairing, and a screenshot.
- [ ] 4.8 Add the banners and the alert:
  - **Too long** (`05-error-too-long.png`): "This macro needs P% of a slot. Remove some steps to save it.", with Save off;
  - **Write failed** (`05-error-write-failed.png`): "Couldn't save to the mouse. Nothing on it changed, and your steps are still here." with Try Again;
  - **Unplugged** (`05-error-unplugged.png`): "Mouse disconnected. Plug it back in to save.", with Save and Delete Macro off;
  - **Delete** (`05-delete-confirmation.png`): "Delete "<name>"?", naming the buttons ("G9 in profile 4 plays this macro. After it's deleted, G9 does nothing until you choose something else for it."), with Cancel and Delete.

  Verify each with the fake service (`Test.Set` failures), plus a screenshot.
- [ ] 4.9 Add the Macro group to the Change dialog per `05-change-dialog-after.png`:
  - placed after Keyboard;
  - the radio "Macro" with "Stored on the mouse";
  - a drop-down of the macros, then a separator and "New Macro…", which opens the editor and comes back with the new macro chosen.

  Verify with a screenshot, and a test that Apply sends kind `macro` with the sector.
- [ ] 4.10 Make `button_actions::describe` show "Macro: <name>", or "Macro: Macro in slot N" (`05-buttons-macro-after.png`). Verify with a unit test, and the Buttons screenshot showing G9 (callout 6) as "Macro: Copy line".

## 5. Verification and docs

- [ ] 5.1 Run `sh target/sdk.sh clippy --workspace --all-targets -- -D warnings` and `test --workspace` in the SDK. Done when both pass.
- [ ] 5.2 Design review: run the app against the fake service and capture every board's state with the screenshot hooks. Compare each with its export and note the differences in `evidence/app/review.md`. Done when no difference is left unexplained.
- [ ] 5.3 Hands-on with the user, on the real mouse:
  - record Copy line;
  - put it on G9 in profile 4;
  - press G9 in a text editor;
  - change a pause;
  - delete it while G9 uses it, and check that G9 does nothing afterwards;
  - replug, and check that the macros and names are still there.

  Done when the user confirms each step.
- [ ] 5.4 Update `README.md` (features and a Macros screenshot) and `PRODUCT.md`. Done when both mention mouse macros and the alpha caveat about writing to the mouse's memory.
