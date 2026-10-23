# Proposal

## Why

v1 lights the whole G512 one way at a time. The user wants single keys coloured: a gaming layout that picks out WASD and the keys around it, switching to it automatically when a game starts. They also want the Left Ctrl LED, which is visibly brighter than the rest, evened out. Spike 5 (`evidence/per-key.md`) confirmed that the keyboard's PerKeyLighting feature (`0x8080`, already allowlisted) sets single keys and full 104-key frames in 28 ms. It also showed that a per-key frame and a built-in effect replace each other.

## What Changes

- **Per-key frames in the protocol crate.** "Set keys" (`0x8080` function 3) sends 14 keys per report, and "commit" (function 5) shows them. No other `0x8080` function is used. Key IDs are USB HID keyboard usages, limited to the ANSI 104-key set.
- **A Custom effect in the service.** It shows a saved key map (key → colour), with brightness scaling every key. Custom appears in the effect list next to the built-in effects. It is re-applied after replug and resume like any other state.
- **Two saved key maps:** Custom, and Gaming. Gaming starts out with WASD, Space, Shift, Ctrl, the number row and the arrows in a bright colour, and the other keys dimmed.
- **Switching to the gaming layout while a game runs.** The service watches the GameMode daemon (`com.feralinteractive.GameMode`) on the session bus. It shows the Gaming map while at least one game is registered, and returns to the saved state when the last one ends. A setting turns this on or off, and an explicit change from a front end always wins. The service never starts GameMode itself and never reads keystrokes.
- **Per-key brightness correction.** An optional `corrections.toml` (key → percent), which the user writes and the service only reads, dims single LEDs. The user's Left Ctrl is matched at 85 %. It applies to Custom, Gaming and Static. When a correction exists, Static is sent as a per-key frame so the correction can apply. Breathe, Cycle, Wave and Ripple run in the keyboard's firmware and can't be corrected.
- **A key-map editor in the app.**
  - A drawn ANSI keyboard: click or drag to select keys, then paint a colour.
  - Group buttons (WASD, arrows, F-keys, number row, numpad, modifiers) select those keys in one click.
  - A switch picks which map is being edited, Custom or Gaming.
  - A "Switch to Gaming layout while a game runs" setting.
  - The editor can be used from the keyboard: arrow keys move between keys, and Space selects.

## Capabilities

### New Capabilities

None. Each piece extends an existing capability.

### Modified Capabilities

- `device-protocol`: adds per-key frame sending and the key ID set, and limits `0x8080` to functions 3 and 5.
- `lighting-service`: adds the Custom effect, the saved key maps, per-key brightness correction, the game-time switch, and new D-Bus members for all of these. It changes the D-Bus interface and persisted-state requirements.
- `lighting-app`: adds the key-map editor and the game switch setting. It changes the effect-controls requirement so Custom shows the editor.

## Impact

- **`g512-hidpp`:** a new `perkey` module (frame building, key set), plus support in the fake keyboard.
- **`g512d`:**
  - `state.rs` gets the Custom effect and key maps;
  - `store.rs` gets new `state.toml` fields, still reading v1 files;
  - `dbus.rs` gets new properties and methods; existing members are unchanged, so v1 front ends keep working;
  - `events.rs` gets a GameMode watcher.
- **`g512-control`:** a new key-map widget and editor page, with an ANSI layout table (key positions and sizes).
- **Dependencies:** no new crates expected. The GameMode watcher uses zbus.
- **Not in scope:**
  - ISO and other layouts;
  - per-game layouts;
  - animations driven from the computer (spike 5 shows 35 frames/s is possible);
  - following the keyboard's own Game Mode key, which sends nothing to the computer.
