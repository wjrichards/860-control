# Tasks

## 1. Protocol: per-key frames

- [x] 1.1 Add `Device::request_very_long` (report `0x12`, 60 parameter bytes), sharing reply matching with `request`; verify with unit tests against a scripted transport: the sent bytes are a 64-byte `12 ff <slot> <fn|sw> …` report, and both `0x11` and `0x12` replies match
- [x] 1.2 Add `g512_hidpp::perkey`: `KeyId` (ANSI 104 set, `KeyId::ALL`, `KeyId::new` rejects others such as `0x64`) and `Frame`; verify with unit tests: there are 104 IDs, `0xe0` is accepted and `0x64` and `0x00` are rejected
- [x] 1.3 Implement `send_frame` (8 × function 3 with header `00 01 00 0e`, then function 5 commit; fails on the first HID++ error or timeout); verify that unit tests reproduce spike 5's exact bytes for the all-blue frame and the single Left Ctrl frame
- [x] 1.4 Extend `FakeKeyboard` to accept `0x8080` functions 3 and 5, record committed frames, and reject any other `0x8080` function; verify with a test that `send_frame` on the fake records the frame and that a call to `0x8080` function 1 fails
- [x] 1.5 Add an ignored hardware test that sends a frame (WASD red, the rest dim white) to the real keyboard; verify by running `cargo test -p g512-hidpp -- --ignored` with the user watching

## 2. Service: Custom effect, key maps, corrections

- [x] 2.1 Introduce `Effect { Zone(EffectKind), Custom }` in `state.rs`, with Custom offered only when PerKeyLighting resolved; verify with unit tests: name round-trip, `"Custom"` is rejected on a device without `0x8080`, and v1 effect names are unchanged
- [x] 2.2 Add `KeyMap` (key → Rgb) with the default Gaming map, and frame composition (map → brightness → correction, off for missing keys, rounding as v1); verify with unit tests for the spec scenarios "Custom at half brightness" (`#800000`) and "Left Ctrl evened out" (`#D9D9D9`)
- [x] 2.3 Load `corrections.toml` read-only at startup, ignoring bad entries with a warning; verify with unit tests: a valid file, an unknown key, a percentage of 0 or 150, and a missing file
- [x] 2.4 Extend `state.toml` with `game_switch`, `[custom]` and `[gaming]`, using serde defaults; verify with unit tests that a v1 file loads with defaults, that a round-trip keeps the maps, and that the Gaming map is written only after an edit
- [x] 2.5 Route every apply through one "what is due" function (zone effect, Custom frame, Gaming frame, Static as a frame when corrections exist); verify with `FakeOpener` tests for each branch, and that brightness 0 still sends zone Off
- [x] 2.6 Add D-Bus members `CustomKeys`, `GamingKeys`, `SetKeys`, `GameSwitch`, `SetGameSwitch` and `GameActive`, with Custom in `Effects`; verify with `busctl --user introspect` and with `busctl` calls: `SetKeys custom` with WASD green while Custom is shown changes the keyboard, while `SetKeys gaming` with Static shown leaves the keyboard unchanged, and a v1-style `SetState Static 255 0 0 50 100` still works
- [x] 2.7 Hardware check: with `corrections.toml` set to `e0 = 85`, compare Static white sent as a frame against zone Static (switching back and forth); record whether they look the same in `evidence/`

## 3. Service: gaming layout while a game runs

- [x] 3.1 Add the GameMode watcher (session bus, no property caching, `NameHasOwner` before any read, re-reading `ClientCount` on `GameRegistered`, `GameUnregistered` and owner changes); verify with a unit test of the count → `game_override` state machine, including "SetState during a game clears it" and "switch off clears it"
- [x] 3.2 Verify that GameMode is never activated: with `gamemoded` not running, start `g512d` and confirm with `busctl --user status com.feralinteractive.GameMode` that it is still not running after 30 s
- [x] 3.3 Verify the switch with `gamemode-simulate-game`: switch on and state Ripple → the Gaming map is shown within 1 s of the game registering, and Ripple is back within 1 s of it ending; switch off → no change; record timings from the journal in `evidence/`
- [ ] 3.4 Verify replug and resume during a game: with `gamemode-simulate-game` running a long game (or a real game), re-enumerate the keyboard and confirm the Gaming map returns within 3 s
- [ ] 3.5 Hands-on: launch a Steam game with `gamemoderun %command%`, and confirm with the user that the keyboard switches to the Gaming map and back when the game is closed

## 4. App: key-map editor and game switch

- [x] 4.1 Add `layout.rs` (ANSI key table: ID, label, row, x and width in quarter-key units, limited to the G512's physical keys); verify with a unit test that every ID is in `KeyId::ALL` and none is duplicated
- [x] 4.2 Turn the window into an `AdwNavigationView` with a "Key maps" row opening the editor page; Custom hides the colour and speed rows; verify by AT-SPI that the row is present for every effect and that Custom shows neither colour nor speed
- [x] 4.3 Build the key grid (toggle buttons with swatches, the accessible name "W, red"), click and drag selection, group buttons, colour and Off paint, and the Custom/Gaming switch; verify by AT-SPI that selecting WASD and painting green produces `SetKeys custom` with exactly those four keys, throttled to at most one call per 100 ms during a drag (busctl monitor)
- [ ] 4.4 Make arrow keys move between drawn keys and Space toggle selection; verify by a keyboard-only pass driven through AT-SPI (move to Q, select it, paint it) and by the user's hands-on pass
- [x] 4.5 Add the "Switch to Gaming layout while a game runs" row with an explanation of `gamemoderun %command%`, plus a "Gaming layout active" indicator; verify that toggling it reaches `SetGameSwitch`, and that during `gamemode-simulate-game` the indicator appears and then clears
- [ ] 4.6 Hands-on pass with the user: paint a Custom map and see it on the keyboard, edit Gaming while Ripple is shown (no change), and check the editor follows light/dark style and the accent colour

## 5. Docs and checks

- [x] 5.1 Update the README: Custom and Gaming maps, `corrections.toml` (with the Left Ctrl example), the game switch and `gamemoderun %command%`, and the new `busctl` members; verify by following it for the corrections step on this machine
- [x] 5.2 Install the new service and Flatpak over v1, check that the user's v1 `state.toml` still applies, and add `corrections.toml` with `e0 = 85`; verify with the journal and by eye
- [ ] 5.3 Run `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` in the GNOME 50 SDK, host clippy and tests for `g512-hidpp` and `g512d`, and `openspec validate add-per-key-lighting --strict`; all must pass
