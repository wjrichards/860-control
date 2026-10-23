# Tasks

## 1. Service: lights on and off

- [x] 1.1 Add `lights_on` to `Controller` (D1): `due_output()` returns zone Off when it is false; `set_lights_on(bool)` applies what is due; `set_state` turns it on; game tracking continues while off. Verify with fake-device tests: off then on restores Ripple; SetState while off turns on; a game starting while off stays dark and turning on during the game shows Gaming; replug while off sends Off; SetKeys and SetGameSwitch leave it unchanged
- [x] 1.2 Save `lights_on` in `state.toml` (D1, Migration Plan); verify with store tests that a v1 file and a change-2 file both load with lights on, and that off round-trips
- [x] 1.3 Add the `LightsOn` property and `SetLightsOn(b)` method (D2), emitted with the other properties; verify with a D-Bus test or `busctl` against a test instance that the value round-trips and a `PropertiesChanged` signal carries it
- [x] 1.4 Install the service and check on the keyboard (user OK for the restart): off and on via `busctl`, replug while off stays dark, `gamemode-simulate-game` while off stays dark, and `SetState` while off lights it
- [ ] 1.5 Retry opening the keyboard at startup while permission is denied (up to 15 s), found at the first real login of 4.3: g512d started at 10:46:42.869, the session's ACL arrived at 10:46:43, and the service stayed `permission-denied`. Verify with unit tests of the retry loop and, at the user's next login, a log line showing access arrived and the lighting restored without a restart

## 2. App: Lights switch

- [x] 2.1 Add the "Lights" `AdwSwitchRow` (D6), read as an optional property and hidden without it; verify by AT-SPI that flipping it sends `SetLightsOn` and that a `busctl` change flips the row

## 3. Quick Settings tile

- [x] 3.1 Create `gnome-extension/` with `metadata.json` (uuid `g512-control@g512control.github.io`, shell-version `["50"]`), `extension.js` and a pure `logic.js` (subtitle, menu model, speed clamping, throttle timing) (D3); verify with `gjs -m tests/logic-test.js` in the `dev` toolbox
- [x] 3.2 Write `tests/fake_service.py` (the `Lighting1` interface on a private bus, logging calls, properties changeable on command) and `tests/nested.sh` (D5); verify that the nested Shell starts in the devkit window with the extension enabled, and doesn't touch the real session's bus or dconf
- [x] 3.3 Build the toggle (D4): checked = `LightsOn`, click calls `SetLightsOn`, subtitle and sensitivity follow the effect, `GameActive` and `DeviceState`, hidden while the service is absent; verify in the nested Shell against the fake service (screenshots of lit, off, gaming, disconnected and absent, plus the call log)
- [x] 3.4 Build the menu (D4): header, effect list with the current effect marked, throttled brightness slider, game switch, "Lighting Settings"; verify in the nested Shell that each control makes the expected call, the slider stays within one call per 100 ms with the final value sent, and outside changes update the open menu
- [x] 3.5 Make `disable()` release everything (D4); verify in the nested Shell that disabling and enabling three times leaves no warnings in its log and the tile returns with current values
- [x] 3.6 Add `packaging/install-extension.sh` (pack and install); verify it installs into a temporary `XDG_DATA_HOME` and `gnome-extensions info` there lists the extension

## 4. Docs and real-session check

- [x] 4.1 README: Quick Settings tile section (turn on extensions, install, log out and in, enable), the Lights switch, and the `SetLightsOn` `busctl` example
- [x] 4.2 Workspace checks: clippy `-D warnings` and tests in the GNOME SDK, the `gjs` tests, and `openspec validate --strict` for the change and specs
- [ ] 4.3 Real session with the user, after they turn on extensions, install and log out and in: the tile toggles the keyboard; effects, slider, game switch and "Lighting Settings" work; the app and tile stay in step; the tile follows light and dark style
