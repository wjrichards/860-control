# Proposal: mouse macros

## Why

The G502 HERO can play macros from its own memory: a recorded run of keys, mouse buttons and pauses on one button, which keeps working on other computers and with the app closed. On Linux nothing can make them today; libratbag reads them but doesn't write them. This is the gap change 6 (`add-mouse-buttons`) left open. The mouse has ten empty memory sectors (6–f) that can hold them.

## What Changes

- **Macros on the mouse:**
  - g512-hidpp learns the macro format the mouse reports (format 1): key press and release, mouse buttons, media keys, pauses and play modes.
  - It also learns the button entry that points a button at a macro (`00 sector 00 offset`).
- **Writable sectors 6–f:** macros may be written to sectors 6–f, one macro per sector, with the same safeguards as profiles: a backup first, a CRC, and a read-back check. The directory (sector 0) and ROM are still never written.
- **g512d:**
  - It reads the macros at connect and keeps their names, since the mouse doesn't store names.
  - It saves and deletes macros, and lets a button play one.
  - Deleting a macro first sets the buttons that use it to Disabled.
  - D-Bus gains `MouseMacros`, `MouseMacroSectors`, `SaveMouseMacro`, `DeleteMouseMacro`, the button kind `macro`, and named errors for "too long", "memory full" and "write failed".
- **The app:**
  - **Macros page:** a new **Mouse › Macros** page lists the macros on the mouse with a memory meter, and has an empty state.
  - **Macro editor:** a dialog where you record keys (pauses kept), then edit the steps. You can reorder them, change keys and pause lengths, add mouse-button, media-key and pause steps, pick how the macro plays, and save or delete it.
  - **Change dialog:** it gains a **Macro** choice with a picker that includes "New Macro…".
  - **Callouts:** a button with a macro reads "Macro: <name>".
- **Hardware first:** before anything ships, a read-only spike and one supervised write test confirm the macro format on the user's mouse. Play modes the mouse doesn't honour are left out of the editor.

Nothing existing breaks: every current D-Bus member keeps its name and signature, and buttons the app couldn't decode before are still kept byte for byte.

## Capabilities

### New Capabilities

None. Macros extend the three existing capabilities.

### Modified Capabilities

- `device-protocol`:
  - onboard memory adds macro sectors 6–f as writable;
  - the button table decodes and encodes the macro entry;
  - a new requirement covers the format-1 macro commands.
- `lighting-service`:
  - the D-Bus interface gains the macro members;
  - Mouse buttons gains the `macro` kind;
  - a new requirement covers storing, naming and deleting macros.
- `lighting-app`:
  - the sidebar's Mouse section gains Macros;
  - Mouse button controls gain the Macro choice and the "Macro: <name>" label;
  - new requirements cover the Macros page and the macro editor.

## Impact

- **g512-hidpp:** a new `macros.rs` (Step, PlayMode, encode, decode, size check); `profiles.rs` (a `MacroSector` type limited to 6–f, `ButtonAction::Macro`); `fake.rs` (macro sectors on the fake mouse).
- **g512d:** a new `macros.rs`; changes to `profile_backup.rs` (a one-time backup of 6–f), `store.rs` (macro names), `dbus.rs`, `buttons.rs` and `controller.rs`.
- **App:** new `macros_page.rs` and `macro_editor.rs`; changes to `change_dialog.rs`, `button_actions.rs`, `service.rs` and `window.rs` (a new page id and sidebar row).
- **Tests:** `gnome-extension/tests/fake_service.py` gains the macro members, for screenshots and app tests.
- **Hardware:** writes to sectors 6–f of the user's G502 HERO, only after the spike. The backup file is `mouse-macros-backup.txt` next to the existing profile backup.
- **Design:** Penpot file "G512 Control", boards listed in `design-plan.md` §10, with exports and `audit.md` in `evidence/design/`.
