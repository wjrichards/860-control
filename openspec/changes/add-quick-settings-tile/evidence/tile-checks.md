# Tile checks

## 3.1 Logic

`gjs -m gnome-extension/tests/logic-test.js` in the `dev` toolbox gives 25 ok. The tests cover:
- **Subtitles:** lit, gaming, off, off during a game, Custom, disconnected, no permission, unsupported, and unknown states.
- **Effect choice:** colour and brightness kept, speed clamped up and down, saved speed kept for effects without a speed.
- **Brightness:** rounding and clamping.
- **Throttle, simulated drag from 100 to 20 over one second:** no gap under 100 ms, 9 to 11 sends, final value sent. Also busy and flush behaviour.

## 3.2 Nested Shell harness

`gnome-extension/tests/nested.sh` runs GNOME Shell 50.6 headless, with a 1280×800 virtual monitor, in the `dev` toolbox. It has:
- its own session bus (`dbus-run-session`);
- its own `XDG_RUNTIME_DIR`;
- `GSETTINGS_BACKEND=memory`;
- temporary data, config, cache and state directories.

Two things are installed only into that temporary data directory:
- `tests/fake_service.py`, which owns `io.github.g512control.Lighting1` on the private bus and logs each call with a timestamp;
- `tests/unsafe-helper`, which turns on unsafe mode so the check can use `Eval` and `Screenshot`.

The smoke run printed a private `DBUS_SESSION_BUS_ADDRESS` (`unix:path=/tmp/dbus-…`), not the real session's, and both extensions were `ACTIVE`. No processes or mounts from the run were left afterwards.

The first run left an empty `run/doc` directory: the session's document portal still had it mounted when cleanup ran. The harness now unmounts it before deleting the temporary directory.

## 3.3–3.5 Tile and menu

`tile_checks.sh` gives 29 passed, 0 failed (`nested/tile_checks.out`).

| Check | Result |
|---|---|
| Lit | visible, checked, sensitive, subtitle "Ripple" (`nested/01-lit.png`) |
| Click | `SetLightsOn (False,)`, then unchecked with the subtitle label hidden (`nested/02-off.png`) |
| Click again | `SetLightsOn (True,)`, "Ripple" back |
| `GameActive` | subtitle "Gaming Layout" (`nested/03-gaming.png`) |
| `DeviceState` disconnected or permission-denied | insensitive, subtitle "Disconnected" or "No Permission" (`nested/04-disconnected.png`) |
| Service quits | tile hidden. When it comes back, the tile shows its values again |
| Menu | header "Keyboard Lighting" with the device name; seven effects with Ripple ticked; slider at 100; game switch off; "Lighting Settings" item for `io.github.g512control.Control.desktop` (`nested/06-menu.png`) |
| Choose Wave | `SetState ('Wave', (61, 56, 70), 52, 100)`. Wave is ticked and the subtitle follows. |
| Wave limited to speeds 10–90, saved speed 95 | `SetState ('Wave', (255, 0, 0), 90, 70)` |
| Slider dragged 100 → 20 over 1 s (63 updates) | 11 calls exactly 100 ms apart; the last has brightness 20 (`nested/drag-calls.txt`); the slider then shows 20 |
| Game switch | `SetGameSwitch (True,)`, and the switch shows on |
| Outside change to Static at 40 % | the tick, slider and subtitle follow while the menu is open (`nested/07-menu-after-change.png`) |
| Disable and enable three times | the tile comes back as "Static", with no new warnings or errors in the Shell log |

**Change to the check:** with the lights off, `QuickMenuToggle.subtitle` reads back as `''` even though the tile sets `null`. The Shell still hides the subtitle label, and the screenshot shows the title centred like Dark Style's. So the check tests that the label is hidden rather than the raw property value.

**Not covered here:** actually opening G512 Control from "Lighting Settings". The app isn't installed inside the toolbox. Task 4.3 checks it in the real session.

## 3.6 Install script

`install_check.sh` runs `packaging/install-extension.sh` against a temporary data directory (`install_check.out`). It installs `extension.js`, `logic.js` and `metadata.json` (uuid `g512-control@g512control.github.io`, shell version `["50"]`) and leaves out `tests/`.

The task named `gnome-extensions info` as the check, but `info` needs a running Shell, so the file listing is used instead. The nested run above already showed the Shell loading these files.

`gnome-extensions install` unpacks into the cache directory and then moves the result into the data directory. So the test puts its temporary cache on the same filesystem; `~/.cache` and `~/.local/share` already share one.

An earlier version passed `--extra-source=logic.js` as a relative path, which `gnome-extensions pack` resolves against the current directory. The script now passes an absolute path.

## 4.2 Workspace checks

- `cargo clippy --workspace --all-targets -- -D warnings` in the GNOME 50 SDK: clean.
- `cargo test --workspace` in the SDK: 84 passed (9 + 28 + 47), plus 2 hardware tests skipped as usual. After the login fix (1.5), a rerun in a fresh SDK sandbox gave 87 passed (9 + 28 + 50), with clippy still clean.
- `gjs -m gnome-extension/tests/logic-test.js`: all 25 passed.
- `openspec validate add-quick-settings-tile --strict` passes, and `openspec validate --specs --strict` gives 3 passed.

## 4.3 Real session (open)

**What is verified:**
- After a full logout and login at 10:46:42, the tile loaded in the user's GNOME Shell 50.6: `gnome-extensions info` gives `State: ACTIVE`, with no JS errors or warnings from it in the journal.
- Over AT-SPI, GNOME Shell's tree contains the tile: "Open keyboard lighting menu" (checked, since the lights are on), and a menu header with "Keyboard Lighting" and "G512 RGB MECHANICAL GAMING KEYBOARD".
- GNOME's own keyboard-backlight tile reports brightness `-1` on this machine, so it stays hidden, and the tile is the only "Keyboard" in Quick Settings.

**Found and fixed:**
- The tile's slider had the accessible name "Keyboard Brightness", which is the same as GNOME's own backlight slider. It is now "Keyboard Lighting Brightness" (`07b0b7b`), and the nested checks still pass (29 of 29).
- The login itself exposed the service bug fixed in 1.5.

**Not yet verified: using the tile by hand.** GNOME 50 exposes no AT-SPI actions on Shell widgets, so the tile can't be clicked by script in the real session. The recordings (`record_calls.sh`) caught no calls from GNOME Shell. The 52 calls recorded between 12:11 and 12:19 all came from the app's Flatpak proxy:
- 32 `SetState` (brightness);
- 8 `SetGameSwitch`;
- 12 `SetKeys gaming` (Gaming map edits).

This task stays open until the user does the tile steps: toggle twice; choose Wave, then Ripple; drag the slider; flip the game switch; open Lighting Settings; check that the app follows; and check light and dark style.
